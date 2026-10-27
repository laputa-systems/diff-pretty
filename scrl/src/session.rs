use regex_lite::Regex;
use std::io::{self, Write};
#[cfg(feature = "terminal")]
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
#[cfg(feature = "terminal")]
use std::thread;
#[cfg(feature = "terminal")]
use std::time::{Duration, Instant};

use crate::document::{Document, Range};
use crate::search::{SearchInput, SearchState};
const RESET: &str = "\x1b[0m";
#[cfg(all(feature = "terminal", unix))]
use crate::terminal::{
    RawMode, SignalGuard, read_event_tty, read_event_tty_cancellable, suspend,
    suspend_requested, terminated,
};
#[cfg(feature = "terminal")]
use crate::{ChunkSource, ExitReason, RunOptions};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub rows: usize,
    pub columns: usize,
}

#[derive(Clone, Debug, Default)]
pub struct SessionOptions {
    pub title: String,
    pub search_history: Vec<String>,
    pub wrap: bool,
    pub follow: bool,
    pub filter: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Interrupt,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    Enter,
    Escape,
    Backspace,
    Delete,
    CtrlU,
    Text(char),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Continue { changed: bool },
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourcePull {
    None,
    One,
    UntilMatch,
    All,
}

pub struct Session {
    size: Size,
    options: SessionOptions,
    document: Document,
    source_document: Option<Document>,
    search: SearchState,
    top: usize,
    horizontal_offset: usize,
    wrap: bool,
    follow: bool,
    filter: Option<Regex>,
    active_search_query: Option<String>,
    help: bool,
    finished: bool,
    frame: Vec<u8>,
    search_ranges: Vec<Vec<Range>>,
}

#[cfg(all(feature = "terminal", unix))]
struct KeyReader {
    cancelled: Arc<AtomicBool>,
    receiver: Option<mpsc::Receiver<Event>>,
    worker: Option<thread::JoinHandle<()>>,
}

#[cfg(all(feature = "terminal", unix))]
impl KeyReader {
    fn spawn<R: io::Read + Send + 'static>(mut input: R) -> Self {
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let (sender, receiver) = mpsc::sync_channel(8);
        let worker = thread::spawn(move || loop {
            match read_event_tty_cancellable(&mut input, &worker_cancelled) {
                Ok(Some(event)) => {
                    if sender.send(event).is_err() {
                        break;
                    }
                }
                Ok(None) | Err(_) => break,
            }
        });
        Self {
            cancelled,
            receiver: Some(receiver),
            worker: Some(worker),
        }
    }

    fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Event, mpsc::RecvTimeoutError> {
        match &self.receiver {
            Some(receiver) => receiver.recv_timeout(timeout),
            None => Err(mpsc::RecvTimeoutError::Disconnected),
        }
    }

    fn request_stop(&mut self) {
        self.receiver.take();
        self.cancelled.store(true, Ordering::Relaxed);
    }

    fn join(&mut self) -> io::Result<()> {
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| io::Error::other("pager key reader panicked"))?;
        }
        Ok(())
    }

    fn stop(&mut self) -> io::Result<()> {
        self.request_stop();
        self.join()
    }
}

#[cfg(all(feature = "terminal", unix))]
impl Drop for KeyReader {
    fn drop(&mut self) {
        self.request_stop();
        // Explicit shutdown reports worker panics; Drop only guarantees cleanup on early return.
        let _ = self.join();
    }
}

impl Session {
    pub fn new(size: Size, options: SessionOptions) -> io::Result<Self> {
        let history = options.search_history.clone();
        let wrap = options.wrap;
        let follow = options.follow;
        let filter = compile_filter(&options)?;
        Ok(Self {
            size,
            options,
            document: Document::default(),
            source_document: filter.as_ref().map(|_| Document::default()),
            search: SearchState::with_history(history),
            top: 0,
            horizontal_offset: 0,
            wrap,
            follow,
            filter,
            active_search_query: None,
            help: false,
            finished: false,
            frame: Vec::with_capacity(size.rows.saturating_mul(size.columns).max(1024)),
            search_ranges: Vec::new(),
        })
    }

    #[cfg(all(feature = "terminal", unix))]
    pub(crate) fn from_document(
        document: Document,
        size: Size,
        options: SessionOptions,
    ) -> io::Result<Self> {
        let history = options.search_history.clone();
        let wrap = options.wrap;
        let follow = options.follow;
        let filter = compile_filter(&options)?;
        let (display_document, source_document) = if let Some(pattern) = &filter {
            (document.filtered(pattern), Some(document))
        } else {
            (document, None)
        };
        Ok(Self {
            size,
            options,
            document: display_document,
            source_document,
            search: SearchState::with_history(history),
            top: 0,
            horizontal_offset: 0,
            wrap,
            follow,
            filter,
            active_search_query: None,
            help: false,
            finished: true,
            frame: Vec::with_capacity(size.rows.saturating_mul(size.columns).max(1024)),
            search_ranges: Vec::new(),
        })
    }

    pub fn push_chunk(&mut self, chunk: &str) {
        if !self.finished {
            let last_line = self.document.line_count().checked_sub(1);
            if self.filter.is_some() {
                let previous_lines = self
                    .source_document
                    .as_ref()
                    .expect("filtered sessions retain their source document")
                    .line_count();
                let stale_tail = previous_lines > 0 && {
                    let pattern = self
                        .filter
                        .as_ref()
                        .expect("filtered sessions retain their filter");
                    self.source_document
                        .as_ref()
                        .expect("filtered sessions retain their source document")
                        .line_text(previous_lines - 1)
                        .is_some_and(|text| pattern.is_match(text))
                };
                self.source_document
                    .as_mut()
                    .expect("filtered sessions retain their source document")
                    .append_for_session(chunk);
                if stale_tail {
                    self.document.pop_last_line();
                }
                let selected = self.selected_search_match();
                let pattern = self
                    .filter
                    .as_ref()
                    .expect("filtered sessions retain their filter");
                let source = self
                    .source_document
                    .as_ref()
                    .expect("filtered sessions retain their source document");
                self.document.append_matching_lines(
                    source,
                    previous_lines.saturating_sub(1),
                    pattern,
                );
                self.rebuild_search_after_filter_change(selected);
            } else {
                self.document.append_for_session(chunk);
            }
            if let Some(search) = self.search.session.as_mut() {
                search.invalidate_append(last_line);
            }
            if self.follow {
                self.top = self.max_top();
            }
        }
        if !self.follow {
            self.clamp_top();
        }
    }

    pub fn finish(&mut self) {
        self.finished = true;
        if self.follow {
            self.top = self.max_top();
        } else {
            self.clamp_top();
        }
    }

    pub fn handle(&mut self, event: Event) -> Action {
        if self.help {
            match event {
                Event::Interrupt => return Action::Quit,
                Event::Escape | Event::Text('h' | 'H' | 'q' | 'Q') => {
                    self.help = false;
                    return Action::Continue { changed: true };
                }
                _ => return Action::Continue { changed: false },
            }
        }
        if self.search.input.is_some() {
            match event {
                Event::Interrupt => return Action::Quit,
                Event::Escape => self.search.cancel(),
                Event::Enter => {
                    let query = self.search.input.as_ref().map(|input| input.text.clone());
                    self.search.submit(&self.document, self.finished);
                    if self.search.input.is_none() {
                        self.active_search_query = self.search.session.as_ref().and(query);
                    }
                }
                Event::Backspace => self.search.backspace(),
                Event::Delete => self.search.delete(),
                Event::CtrlU => self.search.clear_input(),
                Event::Left => self.search.move_cursor(false),
                Event::Right => self.search.move_cursor(true),
                Event::Home => self.search.cursor_start(),
                Event::End => self.search.cursor_end(),
                Event::Up => self.search.history_previous(),
                Event::Down => self.search.history_next(),
                Event::Text(character) => self.search.insert(character),
                _ => return Action::Continue { changed: false },
            }
            return Action::Continue { changed: true };
        }
        let changed = match event {
            Event::Interrupt | Event::Text('q' | 'Q') => return Action::Quit,
            Event::Text('/') => {
                self.search.begin(true);
                return Action::Continue { changed: true };
            }
            Event::Text('?') => {
                self.search.begin(false);
                return Action::Continue { changed: true };
            }
            Event::Text('h' | 'H') => {
                self.help = true;
                return Action::Continue { changed: true };
            }
            Event::Text('n') => self.move_match(self.search.forward),
            Event::Text('N') => self.move_match(!self.search.forward),
            Event::Up => {
                if !self.move_match(false) {
                    self.top = self.top.saturating_sub(1);
                }
                true
            }
            Event::Down => {
                if !self.move_match(true) {
                    self.top = self.top.saturating_add(1);
                }
                true
            }
            Event::Text('k') => {
                self.top = self.top.saturating_sub(1);
                true
            }
            Event::Text('j') => {
                self.top = self.top.saturating_add(1);
                true
            }
            Event::Text('b') | Event::PageUp => {
                self.top = self.top.saturating_sub(self.content_rows());
                true
            }
            Event::Text(' ') | Event::PageDown => {
                self.top = self.top.saturating_add(self.content_rows());
                true
            }
            Event::Text('g') | Event::Home => {
                self.top = 0;
                true
            }
            Event::Text('G') | Event::End => {
                self.top = self.max_top();
                true
            }
            Event::Left => {
                self.horizontal_offset = self
                    .horizontal_offset
                    .saturating_sub(self.horizontal_shift());
                true
            }
            Event::Right => {
                self.horizontal_offset = self
                    .horizontal_offset
                    .saturating_add(self.horizontal_shift());
                true
            }
            _ => return Action::Continue { changed: false },
        };
        self.clamp_top();
        Action::Continue { changed }
    }

    pub fn advance(&mut self) -> bool {
        let before = self.top;
        self.ensure_search();
        before != self.top
    }

    pub fn draw<W: Write + ?Sized>(&mut self, output: &mut W) -> io::Result<()> {
        if self.help {
            self.draw_help(output)?;
            return Ok(());
        }
        self.ensure_search();
        let top = self.top;
        let content_rows = self.content_rows();
        let finished = self.finished;
        let ranges = if let Some(session) = self.search.session.as_mut() {
            session.ensure_cache(&self.document, top, content_rows, finished);
            self.search_ranges.resize_with(content_rows, Vec::new);
            for (row, ranges) in self.search_ranges.iter_mut().enumerate() {
                ranges.clear();
                ranges.extend_from_slice(session.ranges(&self.document, top + row));
            }
            Some((top, self.search_ranges.as_slice()))
        } else {
            None
        };
        let command = self.search.input.as_ref().map(|input| {
            let prefix = if self.search.forward { '/' } else { '?' };
            let mut command = String::with_capacity(input.text.len() + 1);
            command.push(' ');
            command.push(prefix);
            command.push_str(&input.text);
            if let Some(error) = &self.search.error {
                command.push_str("  ");
                command.push_str(error);
            }
            command
        });
        self.frame.clear();
        self.document.write_viewport_search(
            &mut self.frame,
            self.top,
            self.size.rows,
            true,
            self.horizontal_offset,
            ranges,
            &self.options.title,
            self.wrap,
            self.size.columns,
            command.as_deref(),
        )?;
        output.write_all(&self.frame)?;
        output.flush()
    }

    fn draw_help<W: Write + ?Sized>(&mut self, output: &mut W) -> io::Result<()> {
        const HELP: &[&str] = &[
            " scrl help",
            "",
            " j/k or arrows       scroll",
            " PgUp/PgDn, b/space  page",
            " g/Home, G/End       jump to top/bottom",
            " Left/Right          shift horizontally",
            " / or ?              search forward/backward",
            " n/N                 next/previous match",
            " h or Escape         close help",
            " q or Ctrl-C         quit",
        ];
        self.frame.clear();
        self.frame.extend_from_slice(b"\x1b[H");
        for row in 0..self.content_rows() {
            self.frame.extend_from_slice(b"\x1b[2K\r");
            if let Some(line) = HELP.get(row) {
                self.frame.extend_from_slice(line.as_bytes());
            }
            if row + 1 < self.content_rows() {
                self.frame.push(b'\n');
            }
        }
        if self.size.rows > 1 {
            self.frame
                .extend_from_slice(b"\n\x1b[2K\r\x1b[7m help  press h or Escape to return ");
            self.frame.extend_from_slice(RESET.as_bytes());
        }
        output.write_all(&self.frame)?;
        output.flush()
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    fn ensure_search(&mut self) {
        let top = self.top;
        let content_rows = self.content_rows();
        let finished = self.finished;
        let max_top = self.max_top();
        if let Some(session) = self.search.session.as_mut() {
            session.ensure_cache(&self.document, top, content_rows, finished);
            if let Some(line) = session.selected_line() {
                self.top = line.saturating_sub(content_rows / 2).min(max_top);
            }
        }
    }

    fn move_match(&mut self, forward: bool) -> bool {
        let finished = self.finished;
        let Some(session) = self.search.session.as_mut() else {
            return false;
        };
        let changed = session.next(&self.document, forward, finished);
        self.ensure_search();
        changed
    }

    fn content_rows(&self) -> usize {
        self.size.rows.saturating_sub(1).max(1)
    }
    fn max_top(&self) -> usize {
        self.document
            .visual_line_count(self.wrap, self.size.columns)
            .saturating_sub(self.content_rows())
    }
    fn clamp_top(&mut self) {
        self.top = self.top.min(self.max_top());
    }
    fn horizontal_shift(&self) -> usize {
        (self.size.columns / 2).max(1)
    }

    fn selected_search_match(&mut self) -> Option<(usize, String, Range)> {
        let (line, match_index) = self.search.session.as_ref()?.selected?;
        let text = self.document.line_text(line)?.to_owned();
        let range = self
            .search
            .session
            .as_mut()?
            .ranges(&self.document, line)
            .get(match_index)
            .copied()?;
        Some((line, text, range))
    }

    fn rebuild_search_after_filter_change(
        &mut self,
        selected: Option<(usize, String, Range)>,
    ) {
        let Some(query) = self.active_search_query.clone() else {
            return;
        };
        let pending_input = self.search.input.clone();
        let pending_error = self.search.error.clone();
        let forward = self.search.forward;
        self.search.begin(forward);
        self.search.input = Some(SearchInput {
            cursor: query.len(),
            text: query,
        });
        self.search.submit(&self.document, self.finished);
        self.search.input = pending_input;
        self.search.error = pending_error;

        let (Some((old_line, old_text, old_range)), Some(search)) =
            (selected, self.search.session.as_mut())
        else {
            return;
        };
        let mut closest = None::<(usize, usize, usize)>;
        for line in 0..self.document.line_count() {
            if self.document.line_text(line) != Some(old_text.as_str()) {
                continue;
            }
            if let Some(match_index) = search
                .ranges(&self.document, line)
                .iter()
                .position(|range| *range == old_range)
            {
                let distance = line.abs_diff(old_line);
                if closest.is_none_or(|(best, _, _)| distance < best) {
                    closest = Some((distance, line, match_index));
                }
            }
        }
        if let Some((_, line, match_index)) = closest {
            search.selected = Some((line, match_index));
        }
    }
}

fn compile_filter(options: &SessionOptions) -> io::Result<Option<Regex>> {
    options
        .filter
        .as_deref()
        .map(Regex::new)
        .transpose()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

pub(crate) fn validate_options(options: &SessionOptions) -> io::Result<()> {
    compile_filter(options).map(drop)
}

// This entry point intentionally contains only the generic runner policy. A
// host can use Session directly for a richer terminal adapter; the built-in
// adapter keeps lifecycle bytes paired and falls back to direct output when a
// controlling terminal cannot be opened.
#[cfg(all(feature = "terminal", unix))]
pub(crate) fn run_terminal<S: ChunkSource>(
    source: S,
    options: RunOptions,
    output: &mut dyn Write,
) -> io::Result<ExitReason> {
    if options.session.follow
        || matches!(
            options.paging,
            crate::PagingMode::Always | crate::PagingMode::Auto
        )
    {
        return run_terminal_live(source, options, output);
    }
    #[cfg(unix)]
    let tty = match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
    {
        Ok(tty) => tty,
        Err(_) => {
            let mut emit = |chunk: &str| output.write_all(chunk.as_bytes());
            source.produce(&mut emit)?;
            output.flush()?;
            return Ok(ExitReason::EndOfInput);
        }
    };
    #[cfg(not(unix))]
    let mut tty = None::<std::fs::File>;

    #[cfg(unix)]
    let size = rustix::termios::tcgetwinsize(&tty)
        .map(|size| Size {
            rows: usize::from(size.ws_row).max(1),
            columns: usize::from(size.ws_col).max(1),
        })
        .unwrap_or(Size {
            rows: 24,
            columns: 80,
        });
    #[cfg(not(unix))]
    let size = Size {
        rows: 24,
        columns: 80,
    };
    let mut session = Session::new(size, options.session)?;
    let mut emit = |chunk: &str| {
        session.push_chunk(chunk);
        Ok(())
    };
    source.produce(&mut emit)?;
    session.finish();
    if matches!(options.paging, crate::PagingMode::Auto)
        && session.document.line_count() <= size.rows
    {
        session.document.write_to(output)?;
        output.flush()?;
        return Ok(ExitReason::EndOfInput);
    }
    #[cfg(unix)]
    let raw = match RawMode::enter(&tty) {
        Ok(raw) => raw,
        Err(_) => {
            session.document.write_to(output)?;
            output.flush()?;
            return Ok(ExitReason::EndOfInput);
        }
    };
    let _signals = SignalGuard::install();
    output.write_all(b"\x1b[?1049h\x1b[?7l\x1b[?25l\x1b[H")?;
    let mut result = session.draw(output);
    if result.is_ok() {
        loop {
            if terminated() {
                break;
            }
            if suspend_requested() {
                suspend(output, &raw)?;
                session.draw(output)?;
                continue;
            }
            match read_event_tty(&mut &tty)? {
                None => break,
                Some(event) => match session.handle(event) {
                    Action::Quit => break,
                    Action::Continue { changed: true } => {
                        if let Err(error) = session.draw(output) {
                            result = Err(error);
                            break;
                        }
                    }
                    Action::Continue { changed: false } => {}
                },
            }
        }
    }
    let cleanup = output
        .write_all(b"\x1b[0m\x1b[?7h\x1b[?25h\x1b[?1049l")
        .and_then(|()| output.flush());
    #[cfg(unix)]
    drop(raw);
    result.and(cleanup).map(|()| ExitReason::EndOfInput)
}

#[cfg(all(feature = "terminal", not(unix)))]
pub(crate) fn run_terminal<S: ChunkSource>(
    source: S,
    _options: RunOptions,
    output: &mut dyn Write,
) -> io::Result<ExitReason> {
    let mut emit = |chunk: &str| output.write_all(chunk.as_bytes());
    source.produce(&mut emit)?;
    output.flush()?;
    Ok(ExitReason::EndOfInput)
}

#[cfg(all(feature = "terminal", unix))]
pub(crate) fn run_retained_terminal(
    document: Document,
    options: RunOptions,
    output: &mut dyn Write,
) -> io::Result<ExitReason> {
    let tty = match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
    {
        Ok(tty) => tty,
        Err(_) => {
            document.write_to(output)?;
            output.flush()?;
            return Ok(ExitReason::EndOfInput);
        }
    };
    run_retained_terminal_with_tty(document, options, output, &tty)
}

#[cfg(all(feature = "terminal", unix))]
pub(crate) fn run_retained_terminal_with_tty(
    document: Document,
    options: RunOptions,
    output: &mut dyn Write,
    tty: &std::fs::File,
) -> io::Result<ExitReason> {
    validate_options(&options.session)?;
    let size = rustix::termios::tcgetwinsize(tty)
        .map(|size| Size {
            rows: usize::from(size.ws_row).max(1),
            columns: usize::from(size.ws_col).max(1),
        })
        .unwrap_or(Size {
            rows: 24,
            columns: 80,
        });
    if matches!(options.paging, crate::PagingMode::Auto) && document.line_count() <= size.rows {
        document.write_to(output)?;
        output.flush()?;
        return Ok(ExitReason::EndOfInput);
    }
    let raw = match RawMode::enter(tty) {
        Ok(raw) => raw,
        Err(_) => {
            document.write_to(output)?;
            output.flush()?;
            return Ok(ExitReason::EndOfInput);
        }
    };
    let mut session = Session::from_document(document, size, options.session)?;
    let _signals = SignalGuard::install();
    let key_tty = tty
        .try_clone()
        .map_err(|error| io::Error::new(error.kind(), format!("clone pager tty: {error}")))?;
    let mut key_reader = KeyReader::spawn(key_tty);
    output.write_all(b"\x1b[?1049h\x1b[?7l\x1b[?25l\x1b[H")?;
    let mut result = session
        .draw(output)
        .map_err(|error| io::Error::new(error.kind(), format!("draw pager output: {error}")));
    while result.is_ok() {
        if terminated() {
            break;
        }
        if suspend_requested() {
            suspend(output, &raw)?;
            result = session.draw(output);
            continue;
        }
        match key_reader.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => match session.handle(event) {
                Action::Quit => break,
                Action::Continue { changed: true } => result = session.draw(output),
                Action::Continue { changed: false } => {}
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if session.advance() {
                    result = session.draw(output);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let cleanup = output
        .write_all(b"\x1b[0m\x1b[?7h\x1b[?25h\x1b[?1049l")
        .and_then(|()| output.flush())
        .map_err(|error| io::Error::new(error.kind(), format!("cleanup pager output: {error}")));
    let key_cleanup = key_reader.stop();
    drop(raw);
    result
        .and(key_cleanup)
        .and(cleanup)
        .map(|()| ExitReason::Quit)
}

#[cfg(all(feature = "terminal", unix))]
enum LoadEvent {
    Chunk(String),
    Finished(io::Result<()>),
}

#[cfg(all(feature = "terminal", unix))]
enum LoadStatus {
    Reading,
    Finished,
    Quit,
}

#[cfg(all(feature = "terminal", unix))]
fn begin_pager<'a>(
    tty: &'a std::fs::File,
    output: &mut dyn Write,
    session: &mut Session,
) -> io::Result<Option<(RawMode<'a>, KeyReader)>> {
    let raw = match RawMode::enter(tty) {
        Ok(raw) => raw,
        Err(_) => return Ok(None),
    };
    let key_tty = tty.try_clone()?;
    let key_reader = KeyReader::spawn(key_tty);
    if let Err(error) = output.write_all(b"\x1b[?1049h\x1b[?7l\x1b[?25l\x1b[H]") {
        return Err(error);
    }
    if let Err(error) = session.draw(output) {
        let _ = output.write_all(b"\x1b[0m\x1b[?7h\x1b[?25h\x1b[?1049l");
        let _ = output.flush();
        return Err(error);
    }
    Ok(Some((raw, key_reader)))
}

#[cfg(all(feature = "terminal", unix))]
fn take_load(session: &mut Session, event: LoadEvent) -> io::Result<LoadStatus> {
    match event {
        LoadEvent::Chunk(chunk) => {
            session.push_chunk(&chunk);
            Ok(LoadStatus::Reading)
        }
        LoadEvent::Finished(result) => {
            result?;
            session.finish();
            Ok(LoadStatus::Finished)
        }
    }
}

#[cfg(all(feature = "terminal", unix))]
fn wait_for_source(
    receiver: &mpsc::Receiver<LoadEvent>,
    session: &mut Session,
    output: &mut dyn Write,
    raw: &RawMode<'_>,
    key_reader: &KeyReader,
) -> io::Result<LoadStatus> {
    loop {
        if terminated() {
            return Ok(LoadStatus::Quit);
        }
        if suspend_requested() {
            suspend(output, raw)?;
            session.draw(output)?;
        }
        match receiver.try_recv() {
            Ok(event) => return take_load(session, event),
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "source stopped before EOF",
                ));
            }
        }
        match key_reader.recv_timeout(Duration::from_millis(15)) {
            Ok(event) => match session.handle(event) {
                Action::Quit => return Ok(LoadStatus::Quit),
                Action::Continue { changed: true } => session.draw(output)?,
                Action::Continue { changed: false } => {}
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "pager key reader stopped",
                ));
            }
        }
    }
}

#[cfg(all(feature = "terminal", unix))]
fn write_unpaged(
    output: &mut dyn Write,
    session: &Session,
    receiver: &mpsc::Receiver<LoadEvent>,
    finished: bool,
) -> io::Result<()> {
    session.document().write_to(output)?;
    if finished {
        output.flush()?;
        return Ok(());
    }
    loop {
        match receiver.recv() {
            Ok(event) => match event {
                LoadEvent::Chunk(chunk) => output.write_all(chunk.as_bytes())?,
                LoadEvent::Finished(result) => {
                    result?;
                    output.flush()?;
                    return Ok(());
                }
            },
            Err(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "source stopped before EOF",
                ));
            }
        }
    }
}

#[cfg(all(feature = "terminal", unix))]
fn run_terminal_live<S: ChunkSource>(
    source: S,
    options: RunOptions,
    output: &mut dyn Write,
) -> io::Result<ExitReason> {
    #[cfg(unix)]
    let tty = match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
    {
        Ok(tty) => tty,
        Err(_) => {
            let mut emit = |chunk: &str| output.write_all(chunk.as_bytes());
            source.produce(&mut emit)?;
            output.flush()?;
            return Ok(ExitReason::EndOfInput);
        }
    };
    #[cfg(not(unix))]
    {
        let mut emit = |chunk: &str| output.write_all(chunk.as_bytes());
        source.produce(&mut emit)?;
        output.flush()?;
        return Ok(ExitReason::EndOfInput);
    }
    #[cfg(unix)]
    let size = rustix::termios::tcgetwinsize(&tty)
        .map(|size| Size {
            rows: usize::from(size.ws_row).max(1),
            columns: usize::from(size.ws_col).max(1),
        })
        .unwrap_or(Size {
            rows: 24,
            columns: 80,
        });
    let follow = options.session.follow;
    let mut session = Session::new(size, options.session)?;
    let mut raw: Option<RawMode<'_>> = None;
    let mut key_reader: Option<KeyReader> = None;
    let mut _signals: Option<SignalGuard> = None;
    let mut pager_started = false;
    if follow {
        match begin_pager(&tty, output, &mut session)? {
            Some((mode, keys)) => {
                raw = Some(mode);
                key_reader = Some(keys);
                _signals = Some(SignalGuard::install());
                pager_started = true;
            }
            None => {
                let mut emit = |chunk: &str| output.write_all(chunk.as_bytes());
                source.produce(&mut emit)?;
                output.flush()?;
                return Ok(ExitReason::EndOfInput);
            }
        }
    }

    let cancelled = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker_cancelled = Arc::clone(&cancelled);
    thread::spawn(move || {
        let result = source.produce(&mut |chunk: &str| {
            if worker_cancelled.load(Ordering::Relaxed) {
                return Err(io::Error::new(io::ErrorKind::Interrupted, "pager quit"));
            }
            sender
                .send(LoadEvent::Chunk(chunk.to_owned()))
                .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "pager stopped reading"))
        });
        let _ = sender.send(LoadEvent::Finished(result));
    });

    let mut result = Ok(());
    let mut finished = false;
    let mut quit = false;
    let mut plain = false;
    let mut last_draw = pager_started.then(Instant::now);

    if follow {
        while result.is_ok() && !finished && !quit {
            let status = {
                let raw = raw.as_ref().expect("pager holds raw mode");
                let key_reader = key_reader.as_ref().expect("pager reads keys");
                wait_for_source(&receiver, &mut session, output, raw, key_reader)
            };
            match status {
                Ok(LoadStatus::Quit) => quit = true,
                Ok(LoadStatus::Finished) => finished = true,
                Ok(LoadStatus::Reading) => {
                    if last_draw.is_none_or(|last| last.elapsed() >= Duration::from_millis(16)) {
                        result = session.draw(output);
                        if result.is_ok() {
                            last_draw = Some(Instant::now());
                        }
                    }
                }
                Err(error) => result = Err(error),
            }
        }
    } else {
        while result.is_ok() && !finished && !pager_started && !plain {
            match receiver.recv() {
                Ok(LoadEvent::Chunk(chunk)) => {
                    session.push_chunk(&chunk);
                    if session.document().line_count() > size.rows {
                        match begin_pager(&tty, output, &mut session) {
                            Ok(Some((mode, keys))) => {
                                raw = Some(mode);
                                key_reader = Some(keys);
                                _signals = Some(SignalGuard::install());
                                pager_started = true;
                            }
                            Ok(None) => {
                                result = write_unpaged(output, &session, &receiver, false);
                                plain = true;
                            }
                            Err(error) => result = Err(error),
                        }
                    }
                }
                Ok(LoadEvent::Finished(source_result)) => {
                    result = source_result;
                    if result.is_ok() {
                        session.finish();
                        finished = true;
                    }
                }
                Err(_) => {
                    result = Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "source stopped before EOF",
                    ));
                }
            }
        }
    }

    if result.is_ok() && !quit && pager_started {
        result = session.draw(output);
        if result.is_ok() {
            last_draw = Some(Instant::now());
        }
    } else if result.is_ok() && !quit && !pager_started && !plain {
        let fits = session.document().line_count() <= size.rows;
        if matches!(options.paging, crate::PagingMode::Auto) && fits {
            session.document().write_to(output)?;
            output.flush()?;
        } else {
            match begin_pager(&tty, output, &mut session) {
                Ok(Some((mode, keys))) => {
                    raw = Some(mode);
                    key_reader = Some(keys);
                    _signals = Some(SignalGuard::install());
                    pager_started = true;
                }
                Ok(None) => {
                    result = write_unpaged(output, &session, &receiver, true);
                }
                Err(error) => result = Err(error),
            }
        }
    }

    while pager_started && result.is_ok() && !quit {
        if terminated() {
            quit = true;
            cancelled.store(true, Ordering::Relaxed);
            break;
        }
        if suspend_requested() {
            let raw = raw.as_ref().expect("pager holds raw mode");
            suspend(output, raw)?;
            result = session.draw(output);
            last_draw = Some(Instant::now());
            continue;
        }
        if session.follow && !finished {
            let status = {
                let raw = raw.as_ref().expect("pager holds raw mode");
                let key_reader = key_reader.as_ref().expect("pager reads keys");
                wait_for_source(&receiver, &mut session, output, raw, key_reader)
            };
            match status {
                Ok(LoadStatus::Quit) => quit = true,
                Ok(LoadStatus::Finished) => {
                    finished = true;
                    result = session.draw(output);
                    if result.is_ok() {
                        last_draw = Some(Instant::now());
                    }
                }
                Ok(LoadStatus::Reading) => {
                    if last_draw.is_none_or(|last| last.elapsed() >= Duration::from_millis(16)) {
                        result = session.draw(output);
                        if result.is_ok() {
                            last_draw = Some(Instant::now());
                        }
                    }
                }
                Err(error) => result = Err(error),
            }
            continue;
        }

        let key_reader = key_reader.as_ref().expect("pager reads keys");
        match key_reader.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => {
                let pull = source_pull_request(&session, finished, event);
                let mut loaded = false;
                let mut changed = false;
                if matches!(pull, SourcePull::UntilMatch) {
                    loop {
                        if result.is_err() || quit {
                            break;
                        }
                        match session.handle(event) {
                            Action::Quit => {
                                quit = true;
                                break;
                            }
                            Action::Continue {
                                changed: event_changed,
                            } => {
                                changed = event_changed;
                                if event_changed || finished {
                                    break;
                                }
                            }
                        }
                        let status = {
                            let raw = raw.as_ref().expect("pager holds raw mode");
                            wait_for_source(&receiver, &mut session, output, raw, key_reader)
                        };
                        match status {
                            Ok(LoadStatus::Reading) => loaded = true,
                            Ok(LoadStatus::Finished) => {
                                loaded = true;
                                finished = true;
                            }
                            Ok(LoadStatus::Quit) => {
                                quit = true;
                                break;
                            }
                            Err(error) => {
                                result = Err(error);
                                break;
                            }
                        }
                    }
                } else if !matches!(pull, SourcePull::None) {
                    loop {
                        let status = {
                            let raw = raw.as_ref().expect("pager holds raw mode");
                            wait_for_source(&receiver, &mut session, output, raw, key_reader)
                        };
                        match status {
                            Ok(LoadStatus::Reading) => loaded = true,
                            Ok(LoadStatus::Finished) => {
                                loaded = true;
                                finished = true;
                            }
                            Ok(LoadStatus::Quit) => {
                                quit = true;
                                break;
                            }
                            Err(error) => {
                                result = Err(error);
                                break;
                            }
                        }
                        if finished || matches!(pull, SourcePull::One) {
                            break;
                        }
                    }
                }
                if result.is_ok() {
                    if !matches!(pull, SourcePull::UntilMatch) {
                        match session.handle(event) {
                            Action::Quit => quit = true,
                            Action::Continue {
                                changed: event_changed,
                            } => changed = event_changed,
                        }
                    }
                    if !quit && (changed || loaded) {
                        result = session.draw(output);
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if finished && session.advance() {
                    result = session.draw(output);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    cancelled.store(true, Ordering::Relaxed);
    let key_cleanup = if let Some(keys) = key_reader.as_mut() {
        keys.stop()
    } else {
        Ok(())
    };
    let cleanup = if pager_started {
        output
            .write_all(b"\x1b[0m\x1b[?7h\x1b[?25h\x1b[?1049l")
            .and_then(|()| output.flush())
    } else {
        Ok(())
    };
    drop(raw);
    result.and(key_cleanup).and(cleanup).map(|()| {
        if quit {
            ExitReason::Quit
        } else {
            ExitReason::EndOfInput
        }
    })
}

fn source_pull_request(session: &Session, finished: bool, event: Event) -> SourcePull {
    if finished {
        return SourcePull::None;
    }
    if session.search.input.is_some() {
        // Search the rendered prefix on submission. A later `n`/`N` can
        // extend that prefix one chunk at a time if the current matches are
        // exhausted.
        return SourcePull::None;
    }
    if session.search.session.is_some() && matches!(event, Event::Text('n' | 'N')) {
        return SourcePull::UntilMatch;
    }
    if matches!(event, Event::Text('G') | Event::End) {
        return SourcePull::All;
    }
    if matches!(
        event,
        Event::Down | Event::Text('j') | Event::PageDown | Event::Text(' ')
    ) && session.top.saturating_add(session.content_rows())
        >= session
            .document
            .visual_line_count(session.wrap, session.size.columns)
    {
        SourcePull::One
    } else {
        SourcePull::None
    }
}

#[cfg(all(feature = "terminal", not(unix)))]
fn run_terminal_live<S: ChunkSource>(
    source: S,
    _options: RunOptions,
    output: &mut dyn Write,
) -> io::Result<ExitReason> {
    let mut emit = |chunk: &str| output.write_all(chunk.as_bytes());
    source.produce(&mut emit)?;
    output.flush()?;
    Ok(ExitReason::EndOfInput)
}

impl Document {
    pub(crate) fn append_for_session(&mut self, chunk: &str) {
        self.append(chunk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct WriteCounter {
        writes: usize,
        flushes: usize,
    }

    impl Write for WriteCounter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.writes += 1;
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flushes += 1;
            Ok(())
        }
    }

    fn session() -> Session {
        Session::new(
            Size {
                rows: 4,
                columns: 40,
            },
            SessionOptions {
                title: "test".into(),
                search_history: Vec::new(),
                wrap: false,
                follow: false,
                filter: None,
            },
    )
    .unwrap_or_else(|error| panic!("valid test options: {error}"))
    }

    #[cfg(all(feature = "terminal", unix))]
    #[test]
    fn key_reader_cancellation_unblocks_and_joins_the_worker() {
        struct WaitingReader {
            started: mpsc::Sender<()>,
            release: mpsc::Receiver<()>,
        }

        impl io::Read for WaitingReader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                self.started.send(()).unwrap();
                self.release.recv().unwrap();
                Ok(0)
            }
        }

        let (started_sender, started_receiver) = mpsc::channel();
        let (release_sender, release_receiver) = mpsc::channel();
        let mut key_reader = KeyReader::spawn(WaitingReader {
            started: started_sender,
            release: release_receiver,
        });
        started_receiver.recv().unwrap();

        key_reader.request_stop();
        release_sender.send(()).unwrap();
        key_reader.join().unwrap();

        assert!(key_reader.worker.is_none());
    }

    #[test]
    fn source_pull_keeps_search_lazy_until_navigation_needs_more() {
        let mut pager = session();
        pager.push_chunk("one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\n");

        assert_eq!(
            source_pull_request(&pager, false, Event::Text('j')),
            SourcePull::None
        );

        pager.handle(Event::End);
        assert_eq!(
            source_pull_request(&pager, false, Event::Text('j')),
            SourcePull::One
        );
        assert_eq!(
            source_pull_request(&pager, false, Event::End),
            SourcePull::All
        );

        pager.handle(Event::Text('/'));
        pager.handle(Event::Text('o'));
        assert_eq!(
            source_pull_request(&pager, false, Event::Enter),
            SourcePull::None
        );
        pager.handle(Event::Enter);
        assert_eq!(
            source_pull_request(&pager, false, Event::Text('n')),
            SourcePull::UntilMatch
        );
    }

    #[test]
    fn query_characters_are_not_commands() {
        let mut pager = session();
        pager.push_chunk("alpha q / beta\n");
        pager.finish();
        pager.handle(Event::Text('/'));
        for c in "q/".chars() {
            pager.handle(Event::Text(c));
        }
        assert!(matches!(
            pager.handle(Event::Enter),
            Action::Continue { changed: true }
        ));
    }

    #[test]
    fn invalid_query_is_editable_and_empty_query_is_inactive() {
        let mut pager = session();
        pager.push_chunk("text\n");
        pager.finish();
        pager.handle(Event::Text('/'));
        pager.handle(Event::Text('['));
        pager.handle(Event::Enter);
        pager.handle(Event::Backspace);
        pager.handle(Event::Enter);
        assert!(pager.search.session.is_none());
    }

    #[test]
    fn search_uses_visible_text() {
        let mut pager = session();
        pager.push_chunk("\x1b[31mred\x1b[0m\n");
        pager.finish();
        pager.handle(Event::Text('/'));
        pager.handle(Event::Text('r'));
        pager.handle(Event::Text('e'));
        pager.handle(Event::Text('d'));
        pager.handle(Event::Enter);
        let mut output = Vec::new();
        pager.draw(&mut output).unwrap();
        assert!(output.windows(3).any(|window| window == b"red"));
    }

    #[test]
    fn draw_emits_one_complete_frame_write() {
        let mut pager = session();
        pager.push_chunk("one\ntwo\n");
        pager.finish();
        let mut output = WriteCounter {
            writes: 0,
            flushes: 0,
        };
        pager.draw(&mut output).unwrap();
        assert_eq!(output.writes, 1);
        assert_eq!(output.flushes, 1);
    }

    #[test]
    fn search_editing_tracks_unicode_cursor_and_history() {
        let mut pager = Session::new(
            Size {
                rows: 4,
                columns: 40,
            },
            SessionOptions {
                title: "test".into(),
                search_history: vec!["previous".into()],
                wrap: false,
                follow: false,
                filter: None,
            },
        )
        .unwrap_or_else(|error| panic!("valid test options: {error}"));
        pager.handle(Event::Text('/'));
        for character in "éx".chars() {
            pager.handle(Event::Text(character));
        }
        pager.handle(Event::Left);
        pager.handle(Event::Text('!'));
        assert_eq!(pager.search.input.as_ref().unwrap().text, "é!x");
        pager.handle(Event::Delete);
        assert_eq!(pager.search.input.as_ref().unwrap().text, "é!");
        pager.handle(Event::Backspace);
        assert_eq!(pager.search.input.as_ref().unwrap().text, "é");
        pager.handle(Event::CtrlU);
        pager.handle(Event::Up);
        assert_eq!(pager.search.input.as_ref().unwrap().text, "previous");
        pager.handle(Event::Down);
        assert_eq!(pager.search.input.as_ref().unwrap().text, "");
    }

    #[test]
    fn reverse_search_and_n_navigation_use_the_same_cached_matches() {
        let mut pager = session();
        pager.push_chunk("alpha\nbeta alpha\nomega\n");
        pager.finish();
        pager.handle(Event::Text('?'));
        for character in "alpha".chars() {
            pager.handle(Event::Text(character));
        }
        pager.handle(Event::Enter);
        assert_eq!(
            pager.search.session.as_ref().unwrap().selected_line(),
            Some(1)
        );
        pager.handle(Event::Text('n'));
        assert_eq!(
            pager.search.session.as_ref().unwrap().selected_line(),
            Some(0)
        );
        pager.handle(Event::Text('N'));
        assert_eq!(
            pager.search.session.as_ref().unwrap().selected_line(),
            Some(1)
        );
    }

    #[test]
    fn next_search_match_scrolls_the_viewport_to_the_selected_line() {
        let mut pager = session();
        pager.push_chunk("alpha\none\ntwo\nthree\nalpha\nfive\n");
        pager.finish();
        pager.handle(Event::Text('/'));
        for character in "alpha".chars() {
            pager.handle(Event::Text(character));
        }
        pager.handle(Event::Enter);

        assert_eq!(pager.top, 0);
        pager.handle(Event::Text('n'));

        assert_eq!(
            pager.search.session.as_ref().unwrap().selected_line(),
            Some(4)
        );
        assert_eq!(pager.top, 3);
    }

    #[test]
    fn unfinished_search_does_not_wrap_before_loading_the_next_chunk() {
        let mut pager = session();
        pager.push_chunk("alpha\none\ntwo\nthree\n");
        pager.handle(Event::Text('/'));
        for character in "alpha".chars() {
            pager.handle(Event::Text(character));
        }
        pager.handle(Event::Enter);

        assert!(matches!(
            pager.handle(Event::Text('n')),
            Action::Continue { changed: false }
        ));
        assert_eq!(
            pager.search.session.as_ref().unwrap().selected_line(),
            Some(0)
        );

        pager.push_chunk("four\nalpha\nfive\n");
        pager.handle(Event::Text('n'));
        assert_eq!(
            pager.search.session.as_ref().unwrap().selected_line(),
            Some(5)
        );
        assert_eq!(pager.top, 4);
    }

    #[test]
    fn unfinished_search_rechecks_a_partial_line_after_append() {
        let mut pager = session();
        pager.push_chunk("alpha\none\nnee");
        pager.handle(Event::Text('/'));
        for character in "needle".chars() {
            pager.handle(Event::Text(character));
        }
        pager.handle(Event::Enter);
        assert_eq!(pager.search.session.as_ref().unwrap().selected_line(), None);

        assert!(matches!(
            pager.handle(Event::Text('n')),
            Action::Continue { changed: false }
        ));
        pager.push_chunk("dle\nalpha\n");
        pager.handle(Event::Text('n'));

        assert_eq!(
            pager.search.session.as_ref().unwrap().selected_line(),
            Some(2)
        );
    }

    #[test]
    fn wrap_mode_scrolls_visual_rows() {
        let mut pager = Session::new(
            Size {
                rows: 4,
                columns: 4,
            },
            SessionOptions {
                title: "test".into(),
                search_history: Vec::new(),
                wrap: true,
                follow: false,
                filter: None,
            },
        )
        .unwrap_or_else(|error| panic!("valid test options: {error}"));
        pager.push_chunk("abcdefghij\nx\n");
        pager.finish();
        assert_eq!(pager.max_top(), 2);
        pager.handle(Event::Down);
        assert_eq!(pager.top, 1);
    }

    #[test]
    fn filter_rebuilds_the_display_without_losing_source_chunks() {
        let mut pager = Session::new(
            Size {
                rows: 4,
                columns: 40,
            },
            SessionOptions {
                title: "test".into(),
                search_history: Vec::new(),
                wrap: false,
                follow: false,
                filter: Some("keep".into()),
            },
        )
        .unwrap_or_else(|error| panic!("valid test options: {error}"));
        pager.push_chunk("drop one\nkeep first\n");
        pager.push_chunk("keep second\ndrop two\n");
        assert_eq!(pager.document.line_count(), 3);
        assert_eq!(pager.document.line_text(0), Some("keep first"));
        assert_eq!(pager.document.line_text(1), Some("keep second"));
        assert_eq!(pager.document.line_text(2), Some(""));
    }

    #[test]
    fn filter_appends_only_new_matches_across_partial_chunks() {
        let mut pager = Session::new(
            Size {
                rows: 4,
                columns: 40,
            },
            SessionOptions {
                title: "test".into(),
                search_history: Vec::new(),
                wrap: false,
                follow: false,
                filter: Some("keep".into()),
            },
        )
        .unwrap_or_else(|error| panic!("valid test options: {error}"));
        pager.push_chunk("drop one\nke");
        assert_eq!(pager.document.line_count(), 0);
        pager.push_chunk("ep first\nkeep second\ndrop two\n");
        assert_eq!(pager.document.line_count(), 3);
        assert_eq!(pager.document.line_text(0), Some("keep first"));
        assert_eq!(pager.document.line_text(1), Some("keep second"));
        assert_eq!(pager.document.line_text(2), Some(""));
    }

    #[test]
    fn filter_discards_a_partial_match_completed_into_a_non_match() {
        let mut pager = Session::new(
            Size {
                rows: 4,
                columns: 40,
            },
            SessionOptions {
                title: "test".into(),
                search_history: Vec::new(),
                wrap: false,
                follow: false,
                filter: Some("^keep$".into()),
            },
        )
        .unwrap_or_else(|error| panic!("valid test options: {error}"));
        pager.push_chunk("keep");
        assert_eq!(pager.document.line_count(), 1);
        assert_eq!(pager.document.line_text(0), Some("keep"));
        pager.push_chunk("s\n");
        assert_eq!(pager.document.line_count(), 0);
    }

    #[test]
    fn filter_matches_single_push_after_many_chunks() {
        let options = SessionOptions {
            title: "test".into(),
            search_history: Vec::new(),
            wrap: false,
            follow: false,
            filter: Some("keep".into()),
        };
        let size = Size {
            rows: 4,
            columns: 40,
        };
        let mut input = String::new();
        for line in 0..200 {
            input.push_str(&format!(
                "line {line} {}\n",
                if line % 3 == 0 { "keep" } else { "drop" }
            ));
        }
        let mut whole = Session::new(size, options.clone())
            .unwrap_or_else(|error| panic!("valid test options: {error}"));
        whole.push_chunk(&input);
        let mut sliced = Session::new(size, options.clone())
            .unwrap_or_else(|error| panic!("valid test options: {error}"));
        let mut index = 0;
        while index < input.len() {
            let end = (index + 13).min(input.len());
            sliced.push_chunk(&input[index..end]);
            index = end;
        }
        assert_eq!(
            sliced.document.line_count(),
            whole.document.line_count()
        );
        for line in 0..whole.document.line_count() {
            assert_eq!(
                sliced.document.line_text(line),
                whole.document.line_text(line),
                "display diverges at line {line}"
            );
        }
    }

    #[test]
    fn filtered_rebuild_preserves_search_selection_highlights_and_next_match() {
        let mut pager = Session::new(
            Size {
                rows: 4,
                columns: 40,
            },
            SessionOptions {
                title: "test".into(),
                search_history: Vec::new(),
                wrap: false,
                follow: false,
                filter: Some("keep".into()),
            },
        )
        .unwrap_or_else(|error| panic!("valid test options: {error}"));
        pager.push_chunk("drop skip\nkeep alpha\nkeep beta\n");
        pager.handle(Event::Text('/'));
        for character in "beta".chars() {
            pager.handle(Event::Text(character));
        }
        pager.handle(Event::Enter);
        assert_eq!(
            pager.search.session.as_ref().unwrap().selected_line(),
            Some(1)
        );

        pager.push_chunk("keep beta later\n");

        let search = pager.search.session.as_mut().expect("search stays active");
        assert_eq!(search.selected_line(), Some(1));
        assert_eq!(search.ranges(&pager.document, 1).len(), 1);
        let mut output = Vec::new();
        pager.draw(&mut output).unwrap();
        assert!(output
            .windows(b"\x1b[48;5;226;30mbeta\x1b[0m".len())
            .any(|window| window == b"\x1b[48;5;226;30mbeta\x1b[0m"));

        pager.handle(Event::Text('n'));
        assert_eq!(
            pager.search.session.as_ref().unwrap().selected_line(),
            Some(2)
        );
    }

    #[test]
    fn invalid_filter_is_returned_as_a_configuration_error() {
        let error = Session::new(
            Size {
                rows: 4,
                columns: 40,
            },
            SessionOptions {
                filter: Some("[".into()),
                ..SessionOptions::default()
            },
        )
        .err()
        .expect("invalid filter must be rejected");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(error.get_ref().is_some());
    }

    #[test]
    fn follow_mode_pins_new_chunks_to_the_bottom() {
        let mut pager = Session::new(
            Size {
                rows: 3,
                columns: 40,
            },
            SessionOptions {
                title: "test".into(),
                search_history: Vec::new(),
                wrap: false,
                follow: true,
                filter: None,
            },
        )
        .unwrap_or_else(|error| panic!("valid test options: {error}"));
        pager.push_chunk("one\ntwo\nthree\n");
        assert_eq!(pager.top, 2);
        pager.push_chunk("four\n");
        assert_eq!(pager.top, 3);
    }

    #[test]
    fn help_is_a_reversible_screen_state() {
        let mut pager = session();
        assert!(matches!(
            pager.handle(Event::Text('h')),
            Action::Continue { changed: true }
        ));
        let mut output = Vec::new();
        pager.draw(&mut output).unwrap();
        assert!(output.windows(9).any(|window| window == b"scrl help"));
        assert!(matches!(
            pager.handle(Event::Escape),
            Action::Continue { changed: true }
        ));
        output.clear();
        pager.draw(&mut output).unwrap();
        assert!(!output.windows(9).any(|window| window == b"scrl help"));
    }
}
