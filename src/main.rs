use once_cell::sync::Lazy;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::path::Path;
use std::process;

static PATH: Lazy<String> = Lazy::new(|| std::env::var("PATH").unwrap());

trait BashQuoting {
    fn process_bash_quoting(&self) -> Vec<String>;
}

impl BashQuoting for &str {
    fn process_bash_quoting(&self) -> Vec<String> {
        // remove literal nothings
        let without_nothing_quote = self.replace("''", "");
        let mut iter = without_nothing_quote.split('\'');

        let unquoted_start = if let Some(start) = iter.next() {
            start
        } else {
            return vec![];
        };

        let mut result = unquoted_start
            .split_whitespace()
            .map(|s| String::from(s))
            .collect::<Vec<String>>();

        // the iterator contains alternating quoted segments and unquoted segments
        while let Some(quoted) = iter.next() {
            result.push(String::from(quoted));
            if let Some(unquoted) = iter.next() {
                for word in unquoted.split_whitespace() {
                    result.push(String::from(word));
                }
            }
        }

        for quoted in iter {
            if !quoted.is_empty() {
                result.push(String::from(quoted));
            }
        }

        result.into()
    }
}

fn main() {
    let stdin = io::stdin();

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut buff = String::new();
        let input = {
            // Wait for user input
            stdin.read_line(&mut buff).unwrap();
            &buff.trim()
        };

        let (command_str, following) = input.split_once(' ').unwrap_or((input, ""));
        let args = following.process_bash_quoting();

        match command_str.try_into() {
            Ok(builtin) => {
                if matches!(builtin, BuiltinCommand::Exit) {
                    break;
                } else {
                    builtin.run_with(args);
                }
            }
            Err(_) => {
                let run_attempt = process::Command::new(command_str).args(args).spawn();
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
    fn run_with(&self, args: Vec<String>) {
        match self {
            BuiltinCommand::Echo => println!("{}", args.join(" ")),
            BuiltinCommand::Type => {
                if let Some(&ref first) = args.first() {
                    if BuiltinCommand::try_from(&first[..]).is_ok() {
                        println!("{first} is a shell builtin");
                        return;
                    }
                    if let Some(path) = first_match_in_path(&first[..]) {
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
