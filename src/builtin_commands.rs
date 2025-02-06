use crate::PATH;
use std::convert::TryFrom;
use std::path::Path;

pub(crate) enum BuiltinCommand {
    Echo,
    Type,
    Exit,
}

impl BuiltinCommand {
    pub(crate) fn run_with<S, I>(&self, args: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str> + ToString,
    {
        let mut args_iter = args.into_iter();
        match self {
            BuiltinCommand::Echo => println!(
                "{}",
                args_iter
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            BuiltinCommand::Type => {
                if let Some(first) = args_iter.next() {
                    let first = first.as_ref();
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

fn first_match_in_path(name: &str) -> Option<Box<Path>> {
    for path_str in PATH.split(':') {
        let path_buf = Path::new(path_str).join(name);
        if path_buf.is_file() {
            return Some(Box::from(path_buf));
        }
    }
    None
}
