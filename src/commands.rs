use anyhow::anyhow;
use std::io;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;

use crate::builtin_commands::BuiltinCommand;
use crate::stream_target::{InStream, OutStream};
use crate::tokens::Operator::{Control, Redirect};
use crate::tokens::{is_shell_blank, ControlOperator, Operator, RedirectOperator, Token, Word};
use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::iter::Peekable;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::str::Chars;

#[derive(Debug)]
pub struct Pipeline {
    inner: Vec<SimpleCommand>,
}

/// a sequence of [`Words`][`crate::tokens::Word`] separated by blanks, terminated by one of
/// the shell’s [`control operators`][`crate::tokens::ControlOperator`]
///
/// [ref](https://www.gnu.org/software/bash/manual/bash.html#Simple-Commands-1)
#[derive(Debug)]
pub struct SimpleCommand {
    pub location: CommandLocation,
    pub args: Box<[Token]>,
    pub stdin: InStream,
    pub stdout: OutStream,
    pub stderr: OutStream,
}

pub struct CommandStream<'a> {
    token_stream: Peekable<TokenStream<'a>>,
}

#[derive(Clone, Debug)]
struct TokenStream<'a> {
    chars: Peekable<Chars<'a>>,
}

/// Progresses the iterator until it reaches the `delimiter`.
/// After returning, `iter` will have progressed passed the delimiter
/// # Ok
/// wraps the progressed slice (excluding the delimiter) in an `Ok`
/// # Err
/// wraps the progressed slice in an `Err` if end of iterator is reached
fn build_quoted(iter: &mut Peekable<Chars>) -> Result<String, String> {
    let original: String = iter.clone().collect();
    let mut build = String::new();
    let delimiter = iter.next().unwrap();

    while let Some(char) = iter.next() {
        match char {
            _ if char == delimiter => return Ok(build),
            '\\' if delimiter == '"' => {
                build.push_str(proccess_escape_in_double_quote(iter).as_str());
            }

            _ => build.push(char),
        }
    }

    Err(original)
}

fn proccess_escape_in_double_quote(iter: &mut Peekable<Chars>) -> String {
    match iter.next() {
        None => {
            todo!("determine what to do when BACKSLASH is the last in the stream")
        }
        Some(c @ ('$' | '\\' | '"')) => c.into(),
        Some(c) => ['\\', c].iter().collect(),
    }
}

pub trait Command {
    fn run_blocking(self) -> io::Result<ExitStatus>
    where
        Self: Sized,
    {
        self.spawn()?.wait()
    }

    fn spawn(self) -> io::Result<ChildHandle>;
}

impl Iterator for TokenStream<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let mut token_builder = String::new();

        while let Some(peeked_char) = self.chars.peek() {
            match peeked_char {
                w if is_shell_blank(w) => {
                    self.chars.next(); // consume the blank
                    if !token_builder.is_empty() {
                        break;
                    }
                }
                meta_c if Operator::may_start_with(meta_c.to_string().as_str()) => {
                    match try_build_operator(&self.chars) {
                        Ok(operator) => {
                            if !token_builder.is_empty() {
                                break;
                            }

                            token_builder = operator;
                            for _ in token_builder.chars() {
                                self.chars.next();
                            } // todo replace for loop with returned clone instead? (requires converting while into a loop)
                            break;
                        }
                        Err(()) => {
                            // consume peeked value
                            token_builder
                                .push(self.chars.next().expect("peeked to confirm is some"));
                        }
                    }
                }
                '\\' => {
                    _ = self.chars.next();
                    if let Some(following) = self.chars.next() {
                        token_builder.push(following);
                    }
                }
                '"' | '\'' => match build_quoted(&mut self.chars) {
                    Ok(s) => token_builder.push_str(&s),
                    Err(ending) => token_builder.push_str(&ending),
                },
                _ => token_builder.push(self.chars.next().expect("peeked to confirm is some")),
            }
        }
        if token_builder.is_empty() {
            return None;
        }
        Some(token_builder.into())
    }
}

// todo fix this mess
fn try_build_operator(iter: &Peekable<Chars>) -> Result<String, ()> {
    let mut iter_clone = iter.clone();
    let mut buf = String::new();

    while let Some(c) = iter_clone.next_if(|c| {
        let potential = format!("{buf}{c}");
        Operator::may_start_with(&potential)
    }) {
        buf.push(c);
    }
    match buf.parse::<Operator>() {
        Ok(_) => Ok(buf),
        Err(()) => Err(()),
    }
}

impl Command for Pipeline {
    fn run_blocking(self) -> io::Result<ExitStatus> {
        log::info!("running {self:?}");
        let mut exit_status = ExitStatus::default();

        // spawn all
        let mut children = Vec::with_capacity(self.inner.len());
        for command in self.inner {
            let child = command.spawn()?;
            children.push(child);
        }

        // wait on all
        for mut child_process in children {
            exit_status = child_process.wait()?;
        }
        Ok(exit_status)
    }

    fn spawn(self) -> io::Result<ChildHandle> {
        todo!()
    }
}

impl Iterator for CommandStream<'_> {
    type Item = Result<Pipeline, anyhow::Error>;

    fn next(&mut self) -> Option<Result<Pipeline, anyhow::Error>> {
        use ControlOperator::{Newline, Pipe, PipeAmp};
        use Token::Operator;

        let mut command_pipeline = Vec::new();

        let mut following_reader = None;

        while let Some(first_token) = self.token_stream.next() {
            let stdin = following_reader.take().unwrap_or(InStream::Std);
            let (mut stdout, mut stderr) = (OutStream::Std, OutStream::Std);

            let location = match CommandLocation::try_from(first_token) {
                Ok(location) => location,
                Err(e) => return Some(Err(e)),
            };

            let mut args = Vec::new();

            'command: loop {
                let Some(token) = self.token_stream.next() else {
                    log::trace!("found end of token stream");
                    break 'command;
                };
                match token {
                    Operator(Control(Newline)) => break 'command,
                    Operator(Redirect(redir)) => {
                        use RedirectOperator as R;

                        log::info!("adding redirect {redir:?}");

                        let Some(Ok(path_buf)) = self.token_stream.next().map(PathBuf::try_from)
                        else {
                            return Some(Err(anyhow!("expected a path after redirect operator")));
                        };

                        let file: File = {
                            match OpenOptions::new()
                                .create(true)
                                .read(true)
                                .write(true)
                                .append(redir.appends())
                                .open(&path_buf)
                            {
                                Ok(f) => f,
                                Err(io_error) => return Some(Err(io_error.into())),
                            }
                        };

                        log::info!("redirect with file: {file:?}");

                        match redir {
                            R::RStdin => todo!("handle [ file < command ]"), // todo error if command_vec.len() > 0 ???
                            R::RStdout | R::AppendStdout => stdout = OutStream::File(file),
                            R::RStderr | R::AppendStderr => stderr = OutStream::File(file),
                        }

                        // // It doesn't make j
                        //
                        // if self
                        //     .token_stream
                        //     .next()
                        //     .is_some_and(|t| !t.is_command_delimiter())
                        // {
                        //     return Some(Err(anyhow!(
                        //         "expected a command delimiter after {redir} {path_buf:?}"
                        //     )));
                        // }
                        // command_pipeline.push(SimpleCommand {
                        //     location,
                        //     args: args.into_boxed_slice(),
                        //     stdin,
                        //     stdout,
                        //     stderr,
                        // });
                        break 'command;
                    }
                    Operator(Control(Pipe)) => {
                        let (reader, writer) = crate::stream_target::pipe();
                        stdout = OutStream::PipeWriter(writer);
                        following_reader = Some(InStream::PipeReader(reader));

                        break 'command;
                    }
                    Operator(Control(PipeAmp)) => todo!(),
                    other_token => args.push(other_token),
                }
            }

            // let mut segment_iter =
            //     (&mut self.token_stream).peeking_take_while(|t| !t.is_control_operator());

            // let (args, redir_option) = collect_until_redir(&mut segment_iter);

            // log::debug!("args: {args:?}");

            let simple_command = SimpleCommand {
                location,
                args: args.into_boxed_slice(),
                stdin,
                stdout,
                stderr,
            };
            command_pipeline.push(simple_command);
        }

        if command_pipeline.is_empty() {
            None
        } else {
            Some(Ok(Pipeline {
                inner: command_pipeline,
            }))
        }
    }
}

#[allow(unused)]
pub fn collect_until_redir<I>(mut iter: I) -> (Box<[Token]>, Option<RedirectOperator>)
where
    I: Iterator<Item = Token>,
{
    let mut collector = Vec::new();
    loop {
        match iter.next() {
            None => return (collector.into_boxed_slice(), None),
            Some(Token::Operator(Operator::Redirect(redir_oper))) => {
                return (collector.into_boxed_slice(), Some(redir_oper));
            }
            Some(other) => collector.push(other),
        }
    }
}

impl<'a, T: AsRef<str>> From<&'a T> for CommandStream<'a> {
    fn from(value: &'a T) -> Self {
        Self {
            token_stream: TokenStream {
                chars: value.as_ref().chars().peekable(),
            }
            .peekable(),
        }
    }
}

impl Command for SimpleCommand {
    fn spawn(self) -> io::Result<ChildHandle> {
        match self.location {
            CommandLocation::Builtin(bltn_command) => Ok(ChildHandle::Completed(
                bltn_command.run_with(self.args, self.stdout, self.stderr)?,
            )),
            CommandLocation::External(external) => {
                let mut command = std::process::Command::new(&*external);
                command.args(self.args);
                command.stdin(self.stdin);
                command.stdout(self.stdout);
                command.stderr(self.stderr);
                match command.spawn() {
                    Ok(child) => Ok(ChildHandle::External(child)),
                    Err(e) if e.kind() == ErrorKind::NotFound => {
                        eprintln!("{}: command not found", external.to_string_lossy());
                        Ok(ChildHandle::Completed(ExitStatus::from_raw(127)))
                    }
                    Err(e) => {
                        log::error!("ERROR SPAWNING PROCESS: {e:?}");
                        todo!("hanlde other error kind")
                    }
                }
            }
        }
    }
}

pub enum ChildHandle {
    Completed(ExitStatus),
    External(std::process::Child),
}

impl ChildHandle {
    fn wait(&mut self) -> io::Result<ExitStatus> {
        match self {
            Self::Completed(exit_status) => {
                // todo this is the for temporary
                Ok(*exit_status)
            }
            Self::External(external) => external.wait(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum CommandLocation {
    Builtin(BuiltinCommand),
    External(Box<Path>),
}

impl TryFrom<&Token> for CommandLocation {
    type Error = anyhow::Error;
    fn try_from(token: &Token) -> Result<Self, Self::Error> {
        if let Token::Word(Word::SimpleWord(string)) = &token {
            return Ok(match string.parse::<BuiltinCommand>() {
                Ok(builtin) => Self::Builtin(builtin),
                Err(..) => Self::External(PathBuf::from(string).into_boxed_path()),
            });
        }

        Err(anyhow!("Token is not a simple word!"))
    }
}
impl TryFrom<Token> for CommandLocation {
    type Error = anyhow::Error;
    fn try_from(token: Token) -> Result<Self, Self::Error> {
        Self::try_from(&token)
    }
}

impl std::fmt::Display for CommandLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let displayed = match self {
            Self::Builtin(builtin_command) => builtin_command.to_string(),
            Self::External(path_buf) => path_buf.to_string_lossy().into_owned(),
        };
        write!(f, "{displayed}")
    }
}
