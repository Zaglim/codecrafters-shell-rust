use std::fs::File;
use std::ops::{Deref, DerefMut};
use std::process::Stdio;

#[derive(Debug)]
pub struct PipeReader {
    inner: imp::PipeReader,
}

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

// todo maybe convert into fd and use a simpler enum
//  note ( the trait bound `std::process::Stdio: From<BorrowedFd<'_>>` is not satisfied )
#[derive(Debug)]
pub enum OutStream {
    Std,
    File(File),
    PipeWriter(PipeWriter),
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

impl From<OutStream> for Stdio {
    fn from(out_stream: OutStream) -> Self {
        match out_stream {
            OutStream::Std => Self::inherit(),
            OutStream::File(f) => f.into(),
            OutStream::PipeWriter(w) => w.inner.into(),
        }
    }
}
