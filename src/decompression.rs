//! Output storage bounded before each write, independently of decoder workspace.
use crate::exceptions::{DecompressionError, OutputLimitExceeded};
use crate::io::RustyBuffer;
use pyo3::PyResult;
use std::io::{self, Write};

#[cfg(any(
    feature = "zstd-pure",
    feature = "lz4-pure",
    feature = "bzip2-pure",
    feature = "deflate-pure",
    feature = "xz-pure"
))]
pub(crate) fn require_unbounded(limit: Option<usize>) -> PyResult<()> {
    if limit.is_some() {
        // libcramjam's pure-Rust decoders can retain decoded output internally
        // before writing to our sink. Do not advertise a memory limit there.
        Err(pyo3::exceptions::PyNotImplementedError::new_err(
            "max_output_size is not supported by this pure-Rust codec",
        ))
    } else {
        Ok(())
    }
}

pub(crate) struct BoundedOutput {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl BoundedOutput {
    pub(crate) fn new(limit: usize, output_len: Option<usize>) -> io::Result<Self> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(output_len.unwrap_or(0).min(limit))
            .map_err(io::Error::other)?;
        Ok(Self {
            bytes,
            limit,
            exceeded: false,
        })
    }

    pub(crate) fn finish<T, E: ToString>(self, result: Result<T, E>) -> PyResult<RustyBuffer> {
        // Some decoders wrap writer errors; retain the specific failure without
        // relying on their error type or text (or on them propagating the error).
        if self.exceeded {
            Err(OutputLimitExceeded::for_limit(self.limit))
        } else {
            result
                .map(|_| RustyBuffer::from(self.bytes))
                .map_err(DecompressionError::from_err)
        }
    }
}

impl Write for BoundedOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit - self.bytes.len() {
            self.exceeded = true;
            return Err(io::Error::other("decompressed output exceeds max_output_size"));
        }
        let needed = self.bytes.len() + bytes.len();
        if needed > self.bytes.capacity() {
            // Grow geometrically without Vec's automatic growth crossing the cap.
            let capacity = needed.max(self.bytes.capacity().saturating_mul(2)).min(self.limit);
            self.bytes
                .try_reserve_exact(capacity - self.bytes.len())
                .map_err(io::Error::other)?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_does_not_preallocate() {
        let output = BoundedOutput::new(usize::MAX, None).unwrap();
        assert_eq!(output.bytes.capacity(), 0);
    }

    #[test]
    fn rejects_write_before_allocating_or_copying() {
        let mut output = BoundedOutput::new(3, None).unwrap();
        output.write_all(b"ab").unwrap();
        let capacity = output.bytes.capacity();
        assert!(output.write_all(b"cd").is_err());
        assert_eq!(output.bytes, b"ab");
        assert_eq!(output.bytes.capacity(), capacity);
        assert!(output.exceeded);
    }

    #[test]
    fn growth_stays_within_limit() {
        let mut output = BoundedOutput::new(37, None).unwrap();
        for _ in 0..37 {
            output.write_all(b"x").unwrap();
            assert!(output.bytes.capacity() <= 37);
        }
        output.write_all(b"").unwrap();
        assert!(output.write_all(b"x").is_err());
        assert_eq!(output.bytes.len(), 37);
    }

    #[test]
    fn hint_is_clamped_and_does_not_pad_output() {
        let mut output = BoundedOutput::new(16, Some(usize::MAX)).unwrap();
        assert_eq!(output.bytes.capacity(), 16);
        output.write_all(b"x").unwrap();
        assert_eq!(output.bytes, b"x");
    }
}
