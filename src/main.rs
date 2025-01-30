use once_cell::unsync::Lazy;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::path::Path;

const PATH: Lazy<String> = Lazy::new(|| std::env::var("PATH").unwrap());
fn main() {
    // PATH.set(std::env::var("PATH").unwrap()).unwrap();

    let stdin = io::stdin();

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut buff = String::new();
        let input = {
            // Wait for user input
            stdin.read_line(&mut buff).unwrap();
            &buff.trim()[..]
        };

        let (command_str, following) = input.split_once(' ').unwrap_or((input, ""));

        match command_str.try_into() {
            Ok(command) => {
                if matches!(command, BuiltinCommand::Exit) {
                    break;
                } else {
                    command.run_with(following);
                }
            }
            Err(_) => {
                println!("{}: command not found", command_str)
            }
        }
    }
}

fn first_match_in_path<'a>(name: &str) -> Option<Box<Path>> {
    for path_str in PATH.split(':') {
        let path = Path::new(path_str).join(name);
        if path.is_file() {
            return Some(Box::from(path));
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
    fn run_with(&self, args_str: &str) {
        match self {
            BuiltinCommand::Echo => println!("{args_str}"),
            BuiltinCommand::Type => {
                if let Ok(_) = BuiltinCommand::try_from(args_str) {
                    println!("{args_str} is a shell builtin");
                } else {
                    if let Some(path) = first_match_in_path(args_str) {
                        println!("{args_str} is {}", path.display());
                    } else {
                        println!("{args_str}: not found")
                    }
                }
            }
            _ => unimplemented!(),
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
