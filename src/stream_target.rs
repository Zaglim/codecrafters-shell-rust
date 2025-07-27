use std::fs::File;
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::process::Stdio;

/// simple wrapper over the implementation
#[derive(Debug)]
pub struct PipeReader {
    inner: imp::PipeReader,
}

/// simple wrapper over the implementation
#[derive(Debug)]
pub struct PipeWriter {
    inner: imp::PipeWriter,
}

pub fn pipe() -> (PipeReader, PipeWriter) {
    let (reader, writer) = imp::pipe().unwrap();
    (PipeReader { inner: reader }, PipeWriter { inner: writer })
}

impl DerefMut for PipeWriter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Deref for PipeWriter {
    type Target = imp::PipeWriter;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// An enum attempting to avoid the need for `dyn Read`
#[derive(Debug)]
pub enum InStream {
    Std,
    File(File),
    PipeReader(PipeReader),
}

mod imp {
    #[rustversion::before(1.87)]
    pub use os_pipe::{pipe, PipeReader, PipeWriter};

    #[rustversion::since(1.87)]
    pub use std::io::{pipe, PipeReader, PipeWriter};
}

/// An enum attempting to avoid the need for `dyn Write`
#[derive(Debug)]
pub enum OutStream<T> {
    Std(T),
    File(File),
    PipeWriter(PipeWriter),
}

impl<W: Write> Write for OutStream<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Std(t) => t.write(buf),
            Self::File(f) => f.write(buf),
            Self::PipeWriter(w) => w.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Std(t) => t.flush(),
            Self::File(f) => f.flush(),
            Self::PipeWriter(w) => w.flush(),
        }
    }
}

impl From<InStream> for Stdio {
    fn from(in_stream: InStream) -> Self {
        match in_stream {
            InStream::Std => Self::inherit(),
            InStream::File(f) => f.into(),
            InStream::PipeReader(w) => w.inner.into(),
        }
    }
}

impl<T> From<OutStream<T>> for Stdio {
    fn from(out_stream: OutStream<T>) -> Self {
        match out_stream {
            OutStream::Std(_) => Self::inherit(),
            OutStream::File(f) => f.into(),
            OutStream::PipeWriter(w) => w.inner.into(),
        }
    }
}
