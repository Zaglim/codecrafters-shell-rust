mod builtin_commands;

use crate::builtin_commands::BuiltinCommand;
use once_cell::sync::Lazy;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::process;
use std::str::{CharIndices, Chars};

static PATH: Lazy<String> = Lazy::new(|| std::env::var("PATH").unwrap());

fn bashify(input: &str) -> std::vec::IntoIter<String> {
    let mut args = Vec::new();
    let mut iter = input.trim().chars();

    let mut arg_builder = String::new();

    while let Some(char) = iter.next() {
        match char {
            _ if char.is_whitespace() => {
                if !arg_builder.is_empty() {
                    args.push(arg_builder);
                    arg_builder = String::new();
                }
            }
            '\\' => {
                if let Some(following) = iter.next() {
                    arg_builder.push(following);
                }
            }
            delim @ ('"' | '\'') => match build_quoted(delim, &mut iter) {
                Ok(s) => arg_builder.push_str(&s),
                Err(ending) => {
                    arg_builder.push(delim);
                    arg_builder.push_str(&ending)
                }
            },
            _ => arg_builder.push(char),
        }
    }
    args.push(arg_builder);
    args.into_iter()
}

/// Progresses the iterator until it reaches the `delimiter`.
/// After returning, `iter` will have progressed passed the delimiter
/// # Ok
/// wraps the progressed slice (excluding the delimiter) in an `Ok`
/// # Err
/// wraps the progressed slice in an `Err` if end of iterator is reached
fn build_quoted(delimiter: char, iter: &mut Chars) -> Result<String, String> {
    let original = iter.as_str();

    let mut build = String::new();

    while let Some(char) = iter.next() {
        match char {
            _ if char == delimiter => return Ok(build),
            '\\' if delimiter == '"' => {
                build.push_str(proccess_escape_in_double_quote(iter).as_str())
            }
            _ => build.push(char),
        }
    }

    Err(original.to_string())
}

fn proccess_escape_in_double_quote(iter: &mut Chars) -> String {
    let mut backslash = String::from('\\');
    match iter.next() {
        None => backslash,
        Some(c@ ('$'|'\\'|'"')) => c.to_string(),
        Some(c) => {
            backslash.push(c);
            backslash
        }
    }
}

fn main() {
    let stdin = io::stdin();

    let mut input_buf = String::new();
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let raw_input = {
            input_buf.clear();
            // Wait for user input
            stdin.read_line(&mut input_buf).unwrap();
            &input_buf
        };

        let mut bash_split = bashify(raw_input);

        let command_str = bash_split.next().unwrap_or_default();
        let args_iter = bash_split;

        match command_str[..].try_into() {
            Ok(BuiltinCommand::Exit) => break,
            Ok(other) => other.run_with(args_iter),
            Err(_) => {
                let run_attempt = process::Command::new(&command_str).args(args_iter).spawn();
                if let Ok(mut child) = run_attempt {
                    child.wait().expect("the child process should have run");
                } else {
                    println!("{}: command not found", command_str);
                }
            }
        }
    }
}
