use anyhow::anyhow;
use std::io;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;

use crate::builtin_commands::BuiltinCommand;
use crate::tokens::Operator::Redirect;
use crate::tokens::{is_shell_blank, Operator, RedirectOperator, Token, Word};
use itertools::Itertools;
use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::iter::Peekable;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::str::Chars;

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
    fn run_blocking(self) -> io::Result<ExitStatus>;
}

pub struct CommandStream<'a> {
    token_stream: TokenStream<'a>,
}

#[derive(Clone, Debug)]
struct TokenStream<'a> {
    chars: Peekable<Chars<'a>>,
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

impl Iterator for CommandStream<'_> {
    type Item = Result<SimpleCommand, anyhow::Error>;

    fn next(&mut self) -> Option<Result<SimpleCommand, anyhow::Error>> {
        let location = match self.token_stream.next().map(CommandLocation::try_from)? {
            Ok(location) => location,
            Err(e) => return Some(Err(e)),
        };
        let mut segment_iter = (&mut self.token_stream)
            .take_while(|t| !t.is_command_delimiter())
            .peekable();

        let args = (&mut segment_iter)
            .peeking_take_while(|token| !token.is_redirect_operator())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        log::debug!("args: {args:?}");

        let (mut stdin, mut stderr, mut stdout) = (None, None, None);
        if let Some(Token::Operator(Operator::Redirect(redir))) = (&mut segment_iter).next() {
            log::info!("adding redirect {redir:?}");

            let Some(Ok(location)) = segment_iter.next().map(PathBuf::try_from) else {
                return Some(Err(anyhow!("expected a path after redirect operator")));
            };

            let file: File = {
                match OpenOptions::new()
                    .create(true)
                    .read(true)
                    .write(true)
                    .append(redir.appends())
                    .open(location)
                {
                    Ok(f) => f,
                    Err(io_error) => return Some(Err(io_error.into())),
                }
            };

            log::info!("redirect with file: {file:?}");

            match redir {
                RedirectOperator::RStdin => stdin = Some(file),
                RedirectOperator::RStdout | RedirectOperator::AppendStdout => {
                    stdout = Some(file);
                }

                RedirectOperator::RStderr | RedirectOperator::AppendStderr => {
                    stderr = Some(file);
                }
            }
        }

        let simple_command = SimpleCommand {
            location,
            args,
            stdin,
            stdout,
            stderr,
        };

        Some(Ok(simple_command))
    }
}

impl<'a, T: AsRef<str>> From<&'a T> for CommandStream<'a> {
    fn from(value: &'a T) -> Self {
        Self {
            token_stream: TokenStream {
                chars: value.as_ref().chars().peekable(),
            },
        }
    }
}

/// a sequence of [`Words`][`crate::tokens::Word`] separated by blanks, terminated by one of the shell’s [`control operators`][`crate::tokens::ControlOperator`]
///
/// [ref](https://www.gnu.org/software/bash/manual/bash.html#Simple-Commands-1)
#[derive(Debug)]
pub struct SimpleCommand {
    pub location: CommandLocation,
    pub args: Box<[Token]>,
    pub stdin: Option<File>,
    pub stdout: Option<File>,
    pub stderr: Option<File>,
}

impl Command for SimpleCommand {
    fn run_blocking(mut self) -> io::Result<ExitStatus> {
        log::trace!("attempting to run {self:?}");
        match self.location {
            CommandLocation::Builtin(bltn_command) => {
                bltn_command.run_with(self.args, self.stdout, self.stderr)
            }
            CommandLocation::External(external) => {
                let mut command = std::process::Command::new(&*external);
                command.args(self.args);

                if let Some(stdin) = self.stdin.take() {
                    command.stdin(stdin);
                }
                if let Some(stdout) = self.stdout {
                    command.stdout(stdout);
                }
                if let Some(stderr) = self.stderr {
                    command.stderr(stderr);
                }
                match command.spawn() {
                    Ok(mut child) => return child.wait(),
                    Err(e) if e.kind() == ErrorKind::NotFound => {
                        eprintln!("{}: command not found", external.to_string_lossy());
                        return Ok(ExitStatus::from_raw(127));
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

#[derive(Clone, Debug)]
pub enum CommandLocation {
    Builtin(BuiltinCommand),
    External(Box<Path>),
}

impl TryFrom<&Token> for CommandLocation {
    type Error = anyhow::Error;
    fn try_from(token: &Token) -> Result<Self, Self::Error> {
        if let Token::Word(Word::SimpleWord(string)) = &token {
            return if let Ok(builtin) = string.parse::<BuiltinCommand>() {
                Ok(Self::Builtin(builtin))
            } else {
                Ok(Self::External(PathBuf::from(string).into_boxed_path()))
            };
        }

        Err(anyhow!("Token is not a simple word!"))
    }
}
impl TryFrom<Token> for CommandLocation {
    type Error = anyhow::Error;
    fn try_from(token: Token) -> Result<Self, Self::Error> {
        CommandLocation::try_from(&token)
    }
}

impl std::fmt::Display for CommandLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let displayed = match self {
            CommandLocation::Builtin(builtin_command) => builtin_command.to_string(),
            CommandLocation::External(path_buf) => path_buf.to_string_lossy().into_owned(),
        };
        write!(f, "{displayed}")
    }
}
