use my_derives::MyFromStrParse;

use crate::builtin_commands::BuiltinCommand;
use crate::tokens::{is_blank, Operator, RedirectOperator, Token};
use std::fs::OpenOptions;
use std::io::Write;
use std::iter::Peekable;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{ExitStatus, Output, Stdio};
use std::str::Chars;

use CommandConstructionError as Cce;
use RedirectOperator as ReO;
use SimpleCommandConstructionError as Scce;

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

#[derive(Debug)]
pub enum Command {
    Simple(SimpleCommand),
}

impl Command {
    pub fn run_blocking(&self) -> anyhow::Result<ExitStatus> {
        match self {
            Command::Simple(simple_command) => simple_command.run_blocking(),
        }
    }
}

impl TryFrom<Vec<Token>> for Command {
    type Error = CommandConstructionError;

    fn try_from(tokens: Vec<Token>) -> Result<Self, Self::Error> {
        if tokens
            .iter()
            .any(|t| matches!(t, Token::Operator(Operator::Control(_))))
        {
            unimplemented!("Compound commands are not implemented")
        }

        match tokens.try_into() {
            Ok(simple) => Ok(Command::Simple(simple)),
            Err(Scce::General(cce)) => Err(cce),
            Err(_) => unimplemented!("compound commands are not implemented"),
        }
    }
}

#[derive(Debug)]
pub enum CommandConstructionError {
    EmptyInput,
}

pub enum SimpleCommandConstructionError {
    General(CommandConstructionError),
    IncludesControlOperator,
}

impl TryFrom<Vec<Token>> for SimpleCommand {
    type Error = SimpleCommandConstructionError;

    fn try_from(tokens: Vec<Token>) -> Result<Self, Self::Error> {
        if tokens
            .iter()
            .any(|t| matches!(t, Token::Operator(Operator::Control(_))))
        {
            return Err(Scce::IncludesControlOperator);
        }

        let (location, args) = tokens.split_first().ok_or(Scce::General(Cce::EmptyInput))?;

        let location = location.into();
        let args = args.into();
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
                            }

                            token_builder = operator;
                            for _ in token_builder.chars() {
                                self.iter.next();
                            } // todo replace for loop with returned clone instead? (requires converting while into a loop)
                            break;
                        }
                        Err(()) => {
                            // consume peeked value
                            token_builder
                                .push(self.iter.next().expect("peeked to confirm is some"));
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
                _ => token_builder.push(self.iter.next().expect("peeked to confirm is some")),
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
    type Item = Result<Command, CommandConstructionError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut token_buff = Vec::new();
        for token in self.iter.by_ref() {
            if token.is_command_delimiter() {
                break;
            }
            token_buff.push(token);
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
pub(crate) struct SimpleCommand {
    pub location: SimpleCommandType,
    pub args: Box<[Token]>,
}

impl SimpleCommand {
    pub fn run_blocking(&self) -> anyhow::Result<ExitStatus> {
        let Some((lhs, operator, path)) = split_first_redirect(&self.args) else {
            return self.run_truly_simple();
        };
        let mut command = std::process::Command::new(self.location.to_string());

        match operator {
            ReO::RStdin => unimplemented!(),

            out_redirect
            @ (ReO::RStdout | ReO::RStderr | ReO::AppendStdout | ReO::AppendStderr) => {
                command.args(lhs.iter().map(ToString::to_string));

                match out_redirect {
                    ReO::RStdout | ReO::AppendStdout => {
                        command.stdout(Stdio::piped());
                    }
                    ReO::RStderr | ReO::AppendStderr => {
                        command.stderr(Stdio::piped());
                    }
                    ReO::RStdin => unreachable!(),
                }

                let Output {
                    status,
                    stdout,
                    stderr,
                } = match command.spawn() {
                    Ok(child) => child.wait_with_output(),
                    Err(e) => return Ok(ExitStatus::from_raw(e.raw_os_error().unwrap_or(-1))),
                }?;

                let mut file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .append(matches!(
                        out_redirect,
                        ReO::AppendStdout | ReO::AppendStderr
                    ))
                    .open(path)?;

                file.write_all(match out_redirect {
                    ReO::RStdout | ReO::AppendStdout => &stdout[..],
                    ReO::RStderr | ReO::AppendStderr => &stderr[..],
                    ReO::RStdin => unreachable!(),
                })?;

                Ok(status)
            }
        }
    }

    fn run_truly_simple(&self) -> anyhow::Result<ExitStatus> {
        match &self.location {
            SimpleCommandType::Builtin(built_in) => built_in.run_with(&self.args),
            SimpleCommandType::External(path) => {
                let mut command = std::process::Command::new(path);
                let run_attempt = command
                    .args(self.args.iter().map(ToString::to_string))
                    .spawn();

                if let Ok(mut child) = run_attempt {
                    child.wait().map_err(Into::into)
                } else {
                    eprintln!("{}: command not found", &path.to_string_lossy());
                    Ok(ExitStatus::from_raw(0))
                }
            }
        }
    }
}

fn split_first_redirect(args: &[Token]) -> Option<(&[Token], ReO, PathBuf)> {
    let redirect_pos = args
        .iter()
        .position(|t| matches!(t, Token::Operator(Operator::Redirect(_))))?;

    let lhs = &args[..redirect_pos];
    let rhs = &args[redirect_pos + 1..];

    let Token::Operator(Operator::Redirect(redirect_token)) = &args[redirect_pos] else {
        panic!()
    };

    match redirect_token {
        ReO::RStdin => unimplemented!(),
        ReO::RStdout | ReO::RStderr | ReO::AppendStdout | ReO::AppendStderr => {
            log::warn!("{rhs:?}");
            assert!(rhs.len() == 1);
            let path: PathBuf = rhs[0].to_string().into();
            Some((lhs, *redirect_token, path))
        }
    }
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
        write!(f, "{displayed}")
    }
}

impl From<&Token> for SimpleCommandType {
    fn from(token: &Token) -> Self {
        if let Ok(builtin) = token.to_string().parse() {
            SimpleCommandType::Builtin(builtin)
        } else {
            let path_buf: PathBuf = token.to_string().into();
            Self::External(path_buf)
        }
    }
}
