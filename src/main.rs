use once_cell::sync::Lazy;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::path::Path;
use std::process;
use std::str::Chars;

static PATH: Lazy<String> = Lazy::new(|| std::env::var("PATH").unwrap());

fn bashify(input: &str) -> Vec<String> {
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
            delim @ ('"' | '\'') => match build_until(delim, &mut iter) {
                Ok(s) => arg_builder.push_str(s),
                Err(ending) => {
                    arg_builder.push(delim);
                    arg_builder.push_str(ending)
                }
            },
            _ => arg_builder.push(char),
        }
    }
    args.push(arg_builder);
    args
}

/// Progresses the iterator until it reaches the `delimiter`.
/// After returning, `iter` will have progressed passed the delimiter
/// # Ok
/// wraps the progressed slice (excluding the delimiter) in an `Ok`
/// # Err
/// wraps the progressed slice in an `Err` if end of iterator is reached
fn build_until<'a>(delimiter: char, iter: &'a mut Chars) -> Result<&'a str, &'a str> {
    let original = iter.as_str();
    let start_index = 0;

    for (index, char) in iter.enumerate() {
        if char == delimiter {
            return Ok(&original[start_index..index]);
        }
    }

    Err(original)
}

fn main() {
    let stdin = io::stdin();

    let mut buff = String::new();
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let raw_input = {
            buff.clear();
            // Wait for user input
            stdin.read_line(&mut buff).unwrap();
            &buff
        };

        let processed: Vec<String> = bashify(raw_input);
        let processed = processed.iter().map(String::as_str).collect::<Vec<_>>();

        let (&command_str, following) = processed.split_first().unwrap_or((&"", &[]));

        match command_str[..].try_into() {
            Ok(BuiltinCommand::Exit) => break,
            Ok(other) => other.run_with(following.to_vec()),
            Err(_) => {
                let run_attempt = process::Command::new(command_str).args(following).spawn();
                if let Ok(mut child) = run_attempt {
                    child.wait().expect("the child process should have run");
                } else {
                    println!("{}: command not found", command_str);
                }
            }
        }
    }
}

fn first_match_in_path(name: &str) -> Option<Box<Path>> {
    for path_str in PATH.split(':') {
        let path_buf = Path::new(path_str).join(name);
        if path_buf.is_file() {
            return Some(Box::from(path_buf));
        }
    }
    None
}

enum BuiltinCommand {
    Echo,
    Type,
    Exit,
}

impl BuiltinCommand {
    fn run_with<'a>(&self, args: impl IntoIterator<Item = &'a str>) {
        let mut args_iter = args.into_iter();
        match self {
            BuiltinCommand::Echo => println!("{}", args_iter.collect::<Vec<_>>().join(" ")),
            BuiltinCommand::Type => {
                if let Some(first) = args_iter.next() {
                    if BuiltinCommand::try_from(first).is_ok() {
                        println!("{first} is a shell builtin");
                        return;
                    }
                    if let Some(path) = first_match_in_path(first) {
                        println!("{} is {}", first, path.display());
                        return;
                    }
                    println!("{first}: not found")
                } else {
                    unimplemented!()
                }
            }
            BuiltinCommand::Exit => unimplemented!(),
        }
    }
}

impl TryFrom<&str> for BuiltinCommand {
    type Error = ();

    fn try_from(value: &str) -> Result<BuiltinCommand, Self::Error> {
        use BuiltinCommand::*;
        match value {
            "echo" => Ok(Echo),
            "type" => Ok(Type),
            "exit" => Ok(Exit),
            _ => Err(()),
        }
    }
}
