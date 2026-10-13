use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

pub trait ChunkSource: Send + 'static {
    fn produce(self, emit: &mut dyn FnMut(&str) -> io::Result<()>) -> io::Result<()>;
}

pub struct ReaderSource<R> {
    reader: R,
    chunk_size: usize,
}

impl<R> ReaderSource<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            chunk_size: 16 * 1024,
        }
    }
}

impl<R: BufRead + Send + 'static> ChunkSource for ReaderSource<R> {
    fn produce(mut self, emit: &mut dyn FnMut(&str) -> io::Result<()>) -> io::Result<()> {
        let mut carry = Vec::with_capacity(4);
        let mut candidate = Vec::new();
        loop {
            let buffer = self.reader.fill_buf()?;
            if buffer.is_empty() {
                return if carry.is_empty() {
                    Ok(())
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        std::str::from_utf8(&carry).expect_err("carry contains incomplete UTF-8"),
                    ))
                };
            }
            let take = buffer.len().min(self.chunk_size);
            if carry.is_empty() {
                match std::str::from_utf8(&buffer[..take]) {
                    Ok(chunk) => emit(chunk)?,
                    Err(error) => {
                        let valid_up_to = error.valid_up_to();
                        if valid_up_to > 0 {
                            let valid = std::str::from_utf8(&buffer[..valid_up_to])
                                .expect("valid_up_to ends at a UTF-8 boundary");
                            emit(valid)?;
                        }
                        if error.error_len().is_some() {
                            return Err(io::Error::new(io::ErrorKind::InvalidData, error));
                        }
                        carry.extend_from_slice(&buffer[valid_up_to..take]);
                    }
                }
            } else {
                candidate.clear();
                candidate.extend_from_slice(&carry);
                candidate.extend_from_slice(&buffer[..take]);
                match std::str::from_utf8(&candidate) {
                    Ok(chunk) => {
                        emit(chunk)?;
                        carry.clear();
                    }
                    Err(error) => {
                        let valid_up_to = error.valid_up_to();
                        if valid_up_to > 0 {
                            let valid = std::str::from_utf8(&candidate[..valid_up_to])
                                .expect("valid_up_to ends at a UTF-8 boundary");
                            emit(valid)?;
                        }
                        if error.error_len().is_some() {
                            return Err(io::Error::new(io::ErrorKind::InvalidData, error));
                        }
                        carry.clear();
                        carry.extend_from_slice(&candidate[valid_up_to..]);
                    }
                }
            }
            self.reader.consume(take);
        }
    }
}

/// A lazily opened seekable file source. The file itself is not opened until
/// the runner starts consuming it, so argument parsing remains side-effect
/// free and a source can be queued behind another source.
pub struct FileSource {
    path: PathBuf,
}

impl FileSource {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_owned(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl ChunkSource for FileSource {
    fn produce(self, emit: &mut dyn FnMut(&str) -> io::Result<()>) -> io::Result<()> {
        let file = std::fs::File::open(&self.path)?;
        ReaderSource::new(BufReader::new(file)).produce(emit)
    }
}

/// Concatenates lazily opened file operands using the same bounded reader as
/// stdin. It deliberately does not insert separators, matching `cat` and
/// preserving the exact bytes supplied by each operand.
pub struct FilesSource {
    paths: Vec<PathBuf>,
}

impl FilesSource {
    pub fn new(paths: impl IntoIterator<Item = impl AsRef<Path>>) -> Self {
        Self {
            paths: paths
                .into_iter()
                .map(|path| path.as_ref().to_owned())
                .collect(),
        }
    }
}

impl ChunkSource for FilesSource {
    fn produce(self, emit: &mut dyn FnMut(&str) -> io::Result<()>) -> io::Result<()> {
        for path in self.paths {
            FileSource::new(path).produce(emit)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ChunkSource, FileSource, FilesSource, ReaderSource};
    use std::fs;
    use std::io::{BufReader, Cursor};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_path(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("scrl-{name}-{}-{stamp}", std::process::id()))
    }

    #[test]
    fn file_sources_stream_operands_without_materializing_them() {
        let first = temporary_path("first");
        let second = temporary_path("second");
        fs::write(&first, "one\né\n").unwrap();
        fs::write(&second, "two\n").unwrap();
        let mut output = String::new();
        FilesSource::new([&first, &second])
            .produce(&mut |chunk| {
                output.push_str(chunk);
                Ok(())
            })
            .unwrap();
        assert_eq!(output, "one\né\ntwo\n");
        fs::remove_file(first).unwrap();
        fs::remove_file(second).unwrap();
    }

    #[test]
    fn missing_file_is_reported_when_source_starts() {
        let path = temporary_path("missing");
        let error = FileSource::new(&path).produce(&mut |_| Ok(())).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn reader_preserves_utf8_scalars_split_at_one_byte_boundaries() {
        let input = "aé𐍈z";
        let reader = BufReader::with_capacity(1, Cursor::new(input.as_bytes()));
        let mut output = String::new();
        ReaderSource::new(reader)
            .produce(&mut |chunk| {
                output.push_str(chunk);
                Ok(())
            })
            .unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn reader_rejects_malformed_and_incomplete_utf8() {
        for input in [&[0xff][..], &[0xe2, 0x82][..]] {
            let reader = BufReader::with_capacity(1, Cursor::new(input));
            let error = ReaderSource::new(reader).produce(&mut |_| Ok(())).unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        }
    }
}
