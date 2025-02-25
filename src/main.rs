mod builtin_commands;
mod commands;
mod tokens;

use crate::commands::{Command, CommandStream, SimpleCommand};
use crate::tokens::*;
use builtin_commands::BuiltinCommand;
use commands::SimpleCommandType;
use log::info;
use once_cell::sync::Lazy;
use std::io::Write;
use std::io::{self};
use std::iter::Peekable;
use std::process::ExitCode;
use std::str::Chars;

pub static PATH: Lazy<Box<str>> = Lazy::new(|| std::env::var("PATH").unwrap().into_boxed_str());

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

thread_local! {
    pub static STDIN: io::Stdin = io::stdin();
}

fn read_line() -> String {
    let mut buf = String::new();
    STDIN.with(|i| i.read_line(&mut buf)).unwrap();
    buf
}

fn main() -> ExitCode {
    #[cfg(debug_assertions)]
    {
        use log::{max_level, LevelFilter};
        colog::default_builder()
            .filter_level(LevelFilter::Trace)
            .init();

        log::log!(
            log::max_level().to_level().unwrap(),
            "logging level = {}",
            max_level()
        );
    }

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let raw_input = read_line();

        info!("{}", dbg!(&raw_input));

        #[cfg(debug_assertions)]
        dbg!(&raw_input);

        for command_construction_result in CommandStream::from(&raw_input).collect::<Vec<_>>() {
            #[cfg(debug_assertions)]
            dbg!(&command_construction_result);

            match command_construction_result {
                Ok(command) => {
                    if matches!(
                        command,
                        Command::Simple(SimpleCommand {
                            location: SimpleCommandType::Builtin(BuiltinCommand::Exit),
                            ..
                        })
                    ) {
                        return ExitCode::SUCCESS;
                    }

                    _ = command.run_blocking();
                }
                Err(commands::CommandConstructionError::NoCommand) => println!(),
            }
        }
    }
}
