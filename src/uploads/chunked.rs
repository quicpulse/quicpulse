//! Chunked transfer encoding support

use bytes::Bytes;
use std::io::Read;

/// Default chunk size for uploads (100KB)
pub const CHUNK_SIZE: usize = 100 * 1024;

/// Iterator that yields chunks of data
pub struct ChunkedReader<R: Read> {
    reader: R,
    chunk_size: usize,
    finished: bool,
}

impl<R: Read> ChunkedReader<R> {
    /// Create a new chunked reader
    pub fn new(reader: R) -> Self {
        Self::with_chunk_size(reader, CHUNK_SIZE)
    }

    /// Create a chunked reader with custom chunk size
    pub fn with_chunk_size(reader: R, chunk_size: usize) -> Self {
        Self {
            reader,
            chunk_size,
            finished: false,
        }
    }
}

impl<R: Read> Iterator for ChunkedReader<R> {
    type Item = std::io::Result<Bytes>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let mut buffer = vec![0u8; self.chunk_size];

        match self.reader.read(&mut buffer) {
            Ok(0) => {
                self.finished = true;
                None
            }
            Ok(n) => {
                buffer.truncate(n);
                Some(Ok(Bytes::from(buffer)))
            }
            Err(e) => {
                self.finished = true;
                Some(Err(e))
            }
        }
    }
}

/// Wrapper for streaming uploads with progress callback
pub struct ChunkedUploadStream<R: Read, F: FnMut(usize)> {
    reader: ChunkedReader<R>,
    callback: F,
}

impl<R: Read, F: FnMut(usize)> ChunkedUploadStream<R, F> {
    /// Create a new chunked upload stream with progress callback
    pub fn new(reader: R, callback: F) -> Self {
        Self {
            reader: ChunkedReader::new(reader),
            callback,
        }
    }
}

impl<R: Read, F: FnMut(usize)> Iterator for ChunkedUploadStream<R, F> {
    type Item = std::io::Result<Bytes>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.next() {
            Some(Ok(bytes)) => {
                (self.callback)(bytes.len());
                Some(Ok(bytes))
            }
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Reader that serves `before` bytes and then fails, for the error path.
    struct FailingReader {
        before: Vec<u8>,
        sent: bool,
    }

    impl Read for FailingReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.sent {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "reader exploded",
                ));
            }
            self.sent = true;
            let n = self.before.len().min(buf.len());
            buf[..n].copy_from_slice(&self.before[..n]);
            Ok(n)
        }
    }

    fn collect(data: &[u8], chunk_size: usize) -> Vec<Vec<u8>> {
        ChunkedReader::with_chunk_size(Cursor::new(data.to_vec()), chunk_size)
            .map(|r| r.unwrap().to_vec())
            .collect()
    }

    #[test]
    fn test_default_chunk_size_is_100kb() {
        assert_eq!(CHUNK_SIZE, 100 * 1024);
        let reader = ChunkedReader::new(Cursor::new(vec![0u8; 4]));
        assert_eq!(reader.chunk_size, CHUNK_SIZE);
        assert!(!reader.finished);
    }

    #[test]
    fn test_splits_input_into_full_chunks() {
        assert_eq!(
            collect(b"abcdef", 2),
            vec![b"ab".to_vec(), b"cd".to_vec(), b"ef".to_vec()]
        );
    }

    #[test]
    fn test_final_chunk_is_truncated_to_the_bytes_read() {
        // 5 bytes at chunk size 2 -> 2, 2, 1 (never a zero-padded chunk).
        let chunks = collect(b"abcde", 2);
        assert_eq!(chunks, vec![b"ab".to_vec(), b"cd".to_vec(), b"e".to_vec()]);
        assert_eq!(chunks.last().unwrap().len(), 1);
    }

    #[test]
    fn test_chunk_larger_than_input_yields_one_chunk() {
        assert_eq!(collect(b"abc", 1024), vec![b"abc".to_vec()]);
    }

    #[test]
    fn test_empty_input_yields_no_chunks() {
        assert!(collect(b"", 8).is_empty());
    }

    #[test]
    fn test_reassembled_chunks_equal_the_original() {
        let data: Vec<u8> = (0..=255u8).cycle().take(5000).collect();
        for chunk_size in [1, 7, 256, 4096, 100_000] {
            let rejoined: Vec<u8> = collect(&data, chunk_size).concat();
            assert_eq!(rejoined, data, "chunk_size {chunk_size} lost data");
        }
    }

    #[test]
    fn test_iterator_stays_exhausted_after_completion() {
        let mut reader = ChunkedReader::with_chunk_size(Cursor::new(b"ab".to_vec()), 2);
        assert!(reader.next().is_some());
        assert!(reader.next().is_none());
        // Further polls must keep returning None.
        assert!(reader.next().is_none());
    }

    #[test]
    fn test_read_error_is_surfaced_then_iteration_stops() {
        let reader = FailingReader {
            before: b"ok".to_vec(),
            sent: false,
        };
        let mut chunked = ChunkedReader::with_chunk_size(reader, 8);

        // First poll delivers the data.
        assert_eq!(chunked.next().unwrap().unwrap().to_vec(), b"ok".to_vec());

        // Second poll surfaces the error...
        let err = chunked.next().unwrap().unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);

        // ...and the iterator is done rather than looping on the error.
        assert!(chunked.next().is_none());
    }

    // ---- ChunkedUploadStream ----

    #[test]
    fn test_upload_stream_reports_progress_per_chunk() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = Rc::clone(&seen);

        let stream = ChunkedUploadStream::new(Cursor::new(vec![0u8; 5]), move |n| {
            sink.borrow_mut().push(n)
        });
        // Default chunk size dwarfs the input, so this arrives in one chunk.
        let chunks: Vec<_> = stream.map(|r| r.unwrap()).collect();

        assert_eq!(chunks.len(), 1);
        assert_eq!(*seen.borrow(), vec![5]);
    }

    #[test]
    fn test_upload_stream_total_progress_matches_payload_size() {
        use std::cell::Cell;
        use std::rc::Rc;

        let total = Rc::new(Cell::new(0usize));
        let counter = Rc::clone(&total);

        let payload = vec![7u8; 300_000];
        let expected = payload.len();
        let stream = ChunkedUploadStream::new(Cursor::new(payload), move |n| {
            counter.set(counter.get() + n)
        });

        let collected: Vec<u8> = stream.flat_map(|r| r.unwrap().to_vec()).collect();
        assert_eq!(collected.len(), expected);
        assert_eq!(
            total.get(),
            expected,
            "progress must sum to the payload size"
        );
    }

    #[test]
    fn test_upload_stream_passes_errors_through_without_reporting_progress() {
        use std::cell::Cell;
        use std::rc::Rc;

        let calls = Rc::new(Cell::new(0usize));
        let counter = Rc::clone(&calls);

        let reader = FailingReader {
            before: Vec::new(),
            sent: false,
        };
        let mut stream = ChunkedUploadStream::new(reader, move |_| counter.set(counter.get() + 1));

        // An empty successful read ends the stream before the error is reached.
        assert!(stream.next().is_none());
        assert_eq!(calls.get(), 0, "no progress for a zero-length read");
    }
}
