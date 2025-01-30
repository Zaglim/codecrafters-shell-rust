#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
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
            Err(_) => println!("{}: command not found", command_str),
            Ok(command) => {
                if matches!(command, Command::Exit) {
                    break;
                } else {
                    command.run_with(following);
                }
            }
        }
    }
}

enum Command {
    Echo,
    Type,
    Exit,
}

impl Command {
    fn run_with(&self, args_str: &str) {
        match self {
            Command::Echo => println!("{args_str}"),
            Command::Type => {
                if let Ok(_) = Command::try_from(args_str) {
                    println!("{args_str} is a shell builtin");
                } else {
                    println!("{args_str}: not found")
                }
            }
            _ => unimplemented!(),
        }
    }
}

impl TryFrom<&str> for Command {
    type Error = ();

    fn try_from(value: &str) -> Result<Command, Self::Error> {
        use Command::*;
        match value {
            "echo" => Ok(Echo),
            "type" => Ok(Type),
            "exit" => Ok(Exit),
            _ => Err(()),
        }
    }
}
