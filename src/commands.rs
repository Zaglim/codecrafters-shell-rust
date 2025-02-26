use my_derives::MyFromStrParse;

use crate::builtin_commands::BuiltinCommand;
use crate::tokens::{is_blank, Operator, RedirectOperator};
use crate::Token;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::iter::Peekable;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{ExitStatus, Output, Stdio};
use std::str::{Chars, FromStr};

type RO = RedirectOperator;

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
                build.push_str(proccess_escape_in_double_quote(iter).as_str())
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

#[derive(Debug)]
pub enum Command {
    Simple(SimpleCommand),
    #[allow(dead_code)]
    Compound(CompoundCommand),
}

impl Command {
    pub fn run_blocking(&self) -> io::Result<ExitStatus> {
        match self {
            Command::Simple(simple_command) => simple_command.run_blocking(),
            Command::Compound(_compound) => todo!(),
        }
    }
}

impl TryFrom<Vec<Token>> for Command {
    type Error = CommandConstructionError;

    fn try_from(tokens: Vec<Token>) -> Result<Self, Self::Error> {
        // todo implement compound commands!
        Ok(Command::Simple(tokens.try_into()?))
    }
}

#[derive(Debug)]
pub enum CommandConstructionError {
    NoCommand,
}

impl TryFrom<Vec<Token>> for SimpleCommand {
    type Error = CommandConstructionError;

    fn try_from(tokens: Vec<Token>) -> Result<Self, Self::Error> {
        let (location, args) = tokens
            .split_first()
            .ok_or(CommandConstructionError::NoCommand)
            .map(|(first, other)| {
                let (_last, args) = other
                    .split_last()
                    .and_then(|(last, args)| {
                        if !last.is_command_delimiter() {
                            None
                        } else {
                            Some((last, args))
                        }
                    })
                    .ok_or(CommandConstructionError::NoCommand)?;

                let location = SimpleCommandType::from(first);
                Ok((location, args.into()))
            })??;

        Ok(Self { location, args })
    }
}

pub struct CommandStream<'a> {
    iter: TokenStream<'a>,
}

struct TokenStream<'a> {
    iter: Peekable<Chars<'a>>,
}

impl Iterator for TokenStream<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let mut token_builder = String::new();

        while let Some(peeked_char) = self.iter.peek() {
            log::trace!("peek: {}", peeked_char);
            match peeked_char {
                w if is_blank(w) => {
                    self.iter.next(); // consume the blank
                    if !token_builder.is_empty() {
                        break;
                    }
                }
                meta_c if Operator::may_start_with(meta_c.to_string().as_str()) => {
                    match try_build_operator(&self.iter) {
                        Ok(operator) => {
                            if !token_builder.is_empty() {
                                break;
                            } else {
                                token_builder = operator;
                                for _ in token_builder.chars() {
                                    self.iter.next();
                                } // todo replace with clone instead
                                break;
                            }
                        }
                        Err(_) => {
                            token_builder.push(self.iter.next().unwrap()); // consume peeked value
                        }
                    }
                }
                '\\' => {
                    _ = self.iter.next();
                    if let Some(following) = self.iter.next() {
                        token_builder.push(following);
                    }
                }
                '"' | '\'' => match build_quoted(&mut self.iter) {
                    Ok(s) => token_builder.push_str(&s),
                    Err(ending) => token_builder.push_str(&ending),
                },
                _ => token_builder.push(self.iter.next().unwrap()),
            }
            log::info!("token_builder = {token_builder}");
        }
        if token_builder.is_empty() {
            return None;
        }
        let token = token_builder.into();
        log::info!("returning token: {token}");

        Some(token)
    }
}

fn try_build_operator(iter: &Peekable<Chars>) -> Result<String, ()> {
    let mut iter_clone = iter.clone();
    let mut buf = String::new();

    while let Some(c) =
        iter_clone.next_if(|c| Operator::may_start_with(&[buf.clone(), c.to_string()].join("")[..]))
    {
        buf += &c.to_string();
    }
    match buf.parse::<Operator>() {
        Ok(_) => Ok(buf),
        Err(_) => Err(()),
    }
}

impl Iterator for CommandStream<'_> {
    type Item = Result<Command, CommandConstructionError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut token_buff = Vec::new();
        for token in self.iter.by_ref() {
            if token.is_command_delimiter() {
                token_buff.push(token);
                break;
            } else {
                token_buff.push(token);
            }
        }
        if token_buff.is_empty() {
            None
        } else {
            Some(Command::try_from(token_buff))
        }
    }
}

impl<'a, T: AsRef<str>> From<&'a T> for CommandStream<'a> {
    fn from(value: &'a T) -> Self {
        Self {
            iter: TokenStream {
                iter: value.as_ref().chars().peekable(),
            },
        }
    }
}

#[derive(Debug)]
pub(crate) struct CompoundCommand {
    // todo
}

#[derive(Debug)]
pub(crate) struct SimpleCommand {
    pub location: SimpleCommandType,
    pub args: Box<[Token]>,
}

impl SimpleCommand {
    pub fn run_blocking(&self) -> io::Result<ExitStatus> {
        if self
            .args
            .iter()
            .any(|t| matches!(t, Token::Operator(Operator::Redirect(_))))
        {
            let (lhs, operator, rhs) = split_redirect(self);
            match operator {
                RO::RStdin => unimplemented!(),

                out_redirect
                @ (RO::RStdout | RO::RStderr | RO::AppendStdout | RO::AppendStderr) => {
                    let path = PathBuf::from_str(&rhs.last().unwrap().to_string()[..]).unwrap();

                    let mut command = std::process::Command::new(lhs.location.to_string());

                    command.args(lhs.args.iter().map(|arg| arg.to_string()));

                    match out_redirect {
                        RO::RStdout | RO::AppendStdout => {
                            command.stdout(Stdio::piped());
                        }
                        RO::RStderr | RO::AppendStderr => {
                            command.stderr(Stdio::piped());
                        }
                        RO::RStdin => unreachable!(),
                    }

                    let output_lhs = match command.spawn() {
                        Ok(child) => child.wait_with_output(),
                        Err(e) => return Ok(ExitStatus::from_raw(e.raw_os_error().unwrap_or(-1))),
                    };

                    match output_lhs {
                        Ok(Output {
                            status,
                            stdout,
                            stderr,
                        }) => {
                            let mut file = OpenOptions::new()
                                .write(true)
                                .create(true)
                                .append(matches!(out_redirect, RO::AppendStdout | RO::AppendStderr))
                                .open(path)
                                .unwrap();
                            file.write_all(match out_redirect {
                                RO::RStdout | RO::AppendStdout => &stdout[..],
                                RO::RStderr | RO::AppendStderr => &stderr[..],
                                RO::RStdin => unreachable!(),
                            })
                            .unwrap();
                            Ok(status)
                        }
                        Err(_io_err) => todo!(),
                    }
                }
            }
        } else {
            self.run_truly_simple()
        }
    }

    fn run_truly_simple(&self) -> io::Result<ExitStatus> {
        match &self.location {
            SimpleCommandType::Builtin(built_in) => built_in.run_with(&self.args),
            SimpleCommandType::External(path) => {
                let mut command = std::process::Command::new(path);
                let run_attempt = command
                    .args(self.args.iter().map(|arg| arg.to_string()))
                    .spawn();

                match run_attempt {
                    Ok(mut child) => child.wait(),
                    Err(_) => {
                        eprintln!("{}: command not found", &path.to_string_lossy());
                        Ok(ExitStatus::from_raw(0))
                    }
                }
            }
        }
    }
}

fn split_redirect(command: &SimpleCommand) -> (SimpleCommand, RedirectOperator, &[Token]) {
    let redirect_pos = command
        .args
        .iter()
        .position(|t| matches!(t, Token::Operator(Operator::Redirect(_))))
        .unwrap();

    let (lhs, rhs) = command.args.split_at(redirect_pos);
    let (Token::Operator(Operator::Redirect(redirect_token)), rhs) = rhs.split_first().unwrap()
    else {
        panic!()
    };
    (
        SimpleCommand {
            location: command.location.clone(),
            args: lhs.into(),
        },
        *redirect_token,
        rhs,
    )
}

#[derive(Clone, MyFromStrParse, Debug)]
pub enum SimpleCommandType {
    Builtin(BuiltinCommand),
    External(PathBuf),
}

impl From<Token> for SimpleCommandType {
    fn from(token: Token) -> Self {
        token.to_string().parse().unwrap()
    }
}

impl std::fmt::Display for SimpleCommandType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let displayed = match self {
            SimpleCommandType::Builtin(builtin_command) => builtin_command.to_string(),
            SimpleCommandType::External(path_buf) => path_buf.to_string_lossy().into_owned(),
        };
        write!(f, "{}", displayed)
    }
}

impl From<&Token> for SimpleCommandType {
    fn from(token: &Token) -> Self {
        match BuiltinCommand::try_from(token.to_string().as_str()) {
            Ok(a) => SimpleCommandType::Builtin(a),
            Err(_) => {
                let path_buf: PathBuf = token.to_string().into();
                Self::External(path_buf)
            }
        }
    }
}
