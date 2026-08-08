//! The intentionally small C ABI used by Git's native diff-pretty adapter.
//!
//! Git owns process state and emits its existing diff-symbol stream. This
//! crate owns only the renderer, retained document, and terminal session.
//! Nothing in the ABI exposes a Rust allocation or a Git-private type.

use diff_pretty::RenderSession;
use diff_pretty::event::DiffEvent;
use scrl::{ChunkSource, ExitReason, PagingMode, RunOptions, SessionOptions};
use std::ffi::{CString, c_char, c_uchar};
use std::io;
use std::ptr;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{
    Arc, mpsc,
    mpsc::{Receiver, SyncSender},
};
use std::thread::{self, JoinHandle};

const ABI_VERSION: u32 = 1;
const STATUS_OK: i32 = 0;
const STATUS_QUIT: i32 = 1;
const STATUS_ERROR: i32 = -1;
const STATUS_INVALID: i32 = -2;
const PAGING_AUTO: u32 = 0;
const PAGING_ALWAYS: u32 = 1;
const PAGING_NEVER: u32 = 2;

#[repr(C)]
pub struct DiffPrettyConfig {
    pub version: u32,
    pub size: u32,
    pub paging: u32,
    pub output_fd: i32,
    pub tty_fd: i32,
}

struct NativeSession {
    renderer: Option<RenderSession>,
    live: Option<LivePager>,
    document: Option<scrl::Document>,
    options: RunOptions,
    output_fd: i32,
    tty_fd: i32,
    error: Option<CString>,
    paged: bool,
    live_finished: bool,
}

pub struct DiffPrettySession {
    inner: NativeSession,
}

fn set_error(session: &mut NativeSession, error: impl std::fmt::Display) -> i32 {
    let message = error.to_string().replace('\0', "?");
    session.error = CString::new(message).ok();
    STATUS_ERROR
}

fn config_size_is_valid(config: &DiffPrettyConfig) -> bool {
    config.size as usize >= std::mem::size_of::<DiffPrettyConfig>()
}

fn paging_mode(value: u32) -> Option<PagingMode> {
    match value {
        PAGING_AUTO => Some(PagingMode::Auto),
        PAGING_ALWAYS => Some(PagingMode::Always),
        PAGING_NEVER => Some(PagingMode::Never),
        _ => None,
    }
}

fn session_mut<'a>(session: *mut DiffPrettySession) -> Result<&'a mut NativeSession, i32> {
    if session.is_null() {
        return Err(STATUS_INVALID);
    }
    // SAFETY: callers receive this pointer only from diff_pretty_begin and
    // must not use it after diff_pretty_abort.
    Ok(unsafe { &mut (*session).inner })
}

enum LiveInput {
    Patch(Vec<u8>),
    Event {
        kind: u32,
        flags: u32,
        data: Vec<u8>,
    },
    Finish,
}

struct LiveSource {
    receiver: Receiver<LiveInput>,
}

impl ChunkSource for LiveSource {
    fn produce(self, emit: &mut dyn FnMut(&str) -> io::Result<()>) -> io::Result<()> {
        let mut renderer = RenderSession::new();
        let mut rendered = String::new();

        loop {
            match self.receiver.recv().map_err(|_| {
                io::Error::new(io::ErrorKind::BrokenPipe, "Git stopped sending pager input")
            })? {
                LiveInput::Patch(data) => {
                    let text = std::str::from_utf8(&data)
                        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                    renderer.push_patch_streaming(text, &mut rendered)?;
                }
                LiveInput::Event { kind, flags, data } => {
                    renderer
                        .push_event_streaming(DiffEvent::new(kind, flags, &data), &mut rendered)?;
                }
                LiveInput::Finish => {
                    renderer.finish_streaming(&mut rendered);
                    if !rendered.is_empty() {
                        emit(&rendered)?;
                    }
                    return Ok(());
                }
            }

            if !rendered.is_empty() {
                emit(&rendered)?;
                rendered.clear();
            }
        }
    }
}

struct LivePager {
    sender: SyncSender<LiveInput>,
    result: Arc<AtomicI32>,
    thread: Option<JoinHandle<io::Result<ExitReason>>>,
}

impl LivePager {
    fn new(options: RunOptions) -> io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(1);
        let result = Arc::new(AtomicI32::new(STATUS_ERROR));
        let thread_result = Arc::clone(&result);
        let thread = thread::Builder::new()
            .name("diff-pretty-pager".into())
            .spawn(move || {
                let result = scrl::run_source(LiveSource { receiver }, options);
                let code = match &result {
                    Ok(ExitReason::EndOfInput) => STATUS_OK,
                    Ok(ExitReason::Quit) => STATUS_QUIT,
                    Err(_) => STATUS_ERROR,
                };
                thread_result.store(code, Ordering::Release);
                result
            })?;
        Ok(Self {
            sender,
            result,
            thread: Some(thread),
        })
    }

    fn send(&self, input: LiveInput) -> Result<(), i32> {
        self.sender.send(input).map_err(|_| {
            if self.result.load(Ordering::Acquire) == STATUS_QUIT {
                STATUS_QUIT
            } else {
                STATUS_ERROR
            }
        })
    }

    fn finish(mut self) -> io::Result<ExitReason> {
        let send_failed = self.sender.send(LiveInput::Finish).is_err();
        let result = self
            .thread
            .take()
            .expect("live pager thread already joined")
            .join()
            .map_err(|_| io::Error::other("diff-pretty pager thread panicked"))??;
        if send_failed && self.result.load(Ordering::Acquire) != STATUS_QUIT {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "pager stopped before Git finished",
            ));
        }
        Ok(result)
    }
}

fn bytes<'a>(data: *const c_uchar, len: usize) -> Result<&'a [u8], i32> {
    if len == 0 {
        return Ok(&[]);
    }
    if data.is_null() {
        return Err(STATUS_INVALID);
    }
    // SAFETY: the caller promises that `data` points to `len` readable bytes
    // for the duration of the call, as specified by the C header.
    Ok(unsafe { std::slice::from_raw_parts(data, len) })
}

fn finish_session(session: &mut NativeSession) -> i32 {
    if session.document.is_some() || session.live_finished {
        return STATUS_OK;
    }
    if let Some(live) = session.live.take() {
        match live.finish() {
            Ok(ExitReason::EndOfInput | ExitReason::Quit) => {
                session.live_finished = true;
                return STATUS_OK;
            }
            Err(error) => return set_error(session, error),
        }
    }
    let Some(renderer) = session.renderer.take() else {
        return set_error(session, "renderer is unavailable");
    };
    session.document = Some(renderer.finish());
    STATUS_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn diff_pretty_begin(
    config: *const DiffPrettyConfig,
) -> *mut DiffPrettySession {
    if config.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: the null check above establishes that the config pointer is
    // readable for the fixed ABI prefix used here.
    let config = unsafe { &*config };
    if config.version != ABI_VERSION || !config_size_is_valid(config) {
        return ptr::null_mut();
    }
    let Some(paging) = paging_mode(config.paging) else {
        return ptr::null_mut();
    };
    if config.output_fd < 0 {
        return ptr::null_mut();
    }
    let options = RunOptions {
        paging,
        session: SessionOptions {
            title: "diff-pretty".into(),
            search_history: Vec::new(),
            wrap: false,
            follow: false,
            filter: None,
        },
    };
    let live = if paging == PagingMode::Always && config.output_fd == 1 {
        match LivePager::new(options.clone()) {
            Ok(live) => Some(live),
            Err(_) => return ptr::null_mut(),
        }
    } else {
        None
    };
    let inner = NativeSession {
        renderer: if live.is_none() {
            Some(RenderSession::new())
        } else {
            None
        },
        live,
        document: None,
        options,
        output_fd: config.output_fd,
        tty_fd: config.tty_fd,
        error: None,
        paged: false,
        live_finished: false,
    };
    Box::into_raw(Box::new(DiffPrettySession { inner }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn diff_pretty_push_patch(
    session: *mut DiffPrettySession,
    data: *const c_uchar,
    len: usize,
) -> i32 {
    let Ok(session) = session_mut(session) else {
        return STATUS_INVALID;
    };
    let Ok(data) = bytes(data, len) else {
        return set_error(session, "invalid patch buffer");
    };
    let text = match std::str::from_utf8(data) {
        Ok(text) => text,
        Err(error) => return set_error(session, error),
    };
    if let Some(live) = session.live.as_ref() {
        return match live.send(LiveInput::Patch(text.as_bytes().to_vec())) {
            Ok(()) => STATUS_OK,
            Err(STATUS_QUIT) => STATUS_QUIT,
            Err(_) => set_error(session, "live pager stopped accepting Git metadata"),
        };
    }
    let Some(renderer) = session.renderer.as_mut() else {
        return set_error(session, "patch submitted after diff_pretty_finish");
    };
    match renderer.push_patch(text) {
        Ok(()) => STATUS_OK,
        Err(error) => set_error(session, error),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn diff_pretty_push_event(
    session: *mut DiffPrettySession,
    kind: u32,
    flags: u32,
    data: *const c_uchar,
    len: usize,
) -> i32 {
    let Ok(session) = session_mut(session) else {
        return STATUS_INVALID;
    };
    let Ok(data) = bytes(data, len) else {
        return set_error(session, "invalid event buffer");
    };
    if let Some(live) = session.live.as_ref() {
        return match live.send(LiveInput::Event {
            kind,
            flags,
            data: data.to_vec(),
        }) {
            Ok(()) => STATUS_OK,
            Err(STATUS_QUIT) => STATUS_QUIT,
            Err(_) => set_error(session, "live pager stopped accepting Git diff events"),
        };
    }
    let Some(renderer) = session.renderer.as_mut() else {
        return set_error(session, "event submitted after diff_pretty_finish");
    };
    match renderer.push_event(DiffEvent::new(kind, flags, data)) {
        Ok(()) => STATUS_OK,
        Err(error) => set_error(session, error),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn diff_pretty_finish(session: *mut DiffPrettySession) -> i32 {
    let Ok(session) = session_mut(session) else {
        return STATUS_INVALID;
    };
    finish_session(session)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn diff_pretty_page(session: *mut DiffPrettySession) -> i32 {
    let Ok(session) = session_mut(session) else {
        return STATUS_INVALID;
    };
    if session.paged {
        return STATUS_OK;
    }
    if session.live_finished {
        session.paged = true;
        return STATUS_OK;
    }
    if session.document.is_none() && finish_session(session) < STATUS_OK {
        return STATUS_ERROR;
    }
    let Some(document) = session.document.as_ref() else {
        return set_error(session, "document is unavailable");
    };
    #[cfg(unix)]
    let result = scrl::run_document_with_fds(
        document,
        session.options.clone(),
        session.output_fd,
        session.tty_fd,
    );
    #[cfg(not(unix))]
    let result = scrl::run_document(document, session.options.clone());
    match result {
        Ok(ExitReason::EndOfInput) => {
            session.paged = true;
            STATUS_OK
        }
        Ok(ExitReason::Quit) => {
            session.paged = true;
            STATUS_QUIT
        }
        Err(error) => set_error(session, error),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn diff_pretty_last_error(
    session: *const DiffPrettySession,
) -> *const c_char {
    if session.is_null() {
        return ptr::null();
    }
    // SAFETY: the pointer is checked above and remains valid until abort.
    let session = unsafe { &(*session).inner };
    session
        .error
        .as_ref()
        .map_or(ptr::null(), |message| message.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn diff_pretty_abort(session: *mut DiffPrettySession) {
    if !session.is_null() {
        // SAFETY: ownership is transferred back exactly once by the C caller.
        let mut session = unsafe { Box::from_raw(session) };
        if let Some(live) = session.inner.live.take() {
            let _ = live.finish();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diff_pretty::event;

    #[test]
    fn semantic_events_can_be_finished_without_serializing_a_document() {
        let config = DiffPrettyConfig {
            version: ABI_VERSION,
            size: std::mem::size_of::<DiffPrettyConfig>() as u32,
            paging: PAGING_NEVER,
            output_fd: 1,
            tty_fd: -1,
        };
        let session = unsafe { diff_pretty_begin(&config) };
        assert!(!session.is_null());
        let header = b"diff --git a/a b/a\n";
        assert_eq!(
            unsafe {
                diff_pretty_push_event(session, event::HEADER, 0, header.as_ptr(), header.len())
            },
            STATUS_OK
        );
        assert_eq!(unsafe { diff_pretty_finish(session) }, STATUS_OK);
        unsafe { diff_pretty_abort(session) };
    }
}
