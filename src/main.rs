use once_cell::sync::Lazy;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::path::Path;
use std::process;
use std::vec::IntoIter;

static PATH: Lazy<String> = Lazy::new(|| std::env::var("PATH").unwrap());

trait BashQuoting {
    fn bashify(&self) -> Vec<&str>;

    fn process_double_quotes(&self) -> IntoIter<&str>;
}

impl BashQuoting for str {
    fn bashify(&self) -> Vec<&str> {
        let mut iter = self.split('\'');

        let unquoted_start: Vec<&str> = if let Some(start) = iter.next() {
            start.process_double_quotes().collect()
        } else {
            return vec![];
        };
        let mut unquoted_end: Vec<&str> = if let Some(end) = iter.next_back() {
            end.process_double_quotes().collect()
        } else {
            return unquoted_start;
        };

        let mut result = unquoted_start;

        // the iterator contains alternating quoted segments and unquoted segments
        while let Some(quoted) = iter.next() {
            result.push(quoted);
            if let Some(unquoted) = iter.next() {
                unquoted
                    .process_double_quotes()
                    .for_each(|word| result.push(word));
            }
        }

        result.append(&mut unquoted_end);

        result
    }

    fn process_double_quotes(&self) -> IntoIter<&str> {
        let mut iter = self.split('"');

        let unquoted_start: Vec<&str> = if let Some(start) = iter.next() {
            start.split_whitespace().collect()
        } else {
            return vec![].into_iter();
        };
        let mut unquoted_end: Vec<&str> = if let Some(end) = iter.next_back() {
            end.split_whitespace().collect()
        } else {
            return unquoted_start.into_iter();
        };

        let mut result = unquoted_start;

        // the iterator contains alternating quoted segments and unquoted segments
        while let Some(quoted) = iter.next() {
            result.push(quoted);
            if let Some(unquoted) = iter.next() {
                unquoted
                    .split_whitespace()
                    .for_each(|word| result.push(word));
            }
        }

        result.append(&mut unquoted_end);

        result.into_iter()
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

        let without_nothing_quote = input.replace("''", "").replace("\"\"", "");
        let a = without_nothing_quote.as_str();

        let processed = a.bashify();
        let mut iter = processed.iter();
        let command_str = iter.next().unwrap();
        let following: Vec<&str> = iter.copied().collect();

        match command_str[..].try_into() {
            Ok(builtin) => {
                if matches!(builtin, BuiltinCommand::Exit) {
                    break;
                } else {
                    builtin.run_with(following);
                }
            }
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
    fn run_with(&self, args: Vec<&str>) {
        match self {
            BuiltinCommand::Echo => println!("{}", args.join(" ")),
            BuiltinCommand::Type => {
                if let Some(first) = args.first() {
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
