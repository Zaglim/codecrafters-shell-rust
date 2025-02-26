use my_derives::MyFromStrParse;
use strum::{Display, EnumIter, IntoStaticStr};

use crate::PATH;
use std::convert::TryFrom;
use std::io;
use std::path::Path;
use std::process::ExitStatus;

/// "A command that is implemented internally by the shell itself, rather than by an executable program somewhere in the file system."
///
/// -- [ref manual](https://www.gnu.org/software/bash/manual/bash.html#index-builtin-1)
#[derive(Clone, MyFromStrParse, IntoStaticStr, strum::Display, Debug, EnumIter)]
pub(crate) enum BuiltinCommand {
    #[strum(serialize = "echo")]
    Echo,
    #[strum(serialize = "type")]
    Type,
    #[strum(serialize = "exit")]
    Exit,
}

#[derive(Debug, Display)]
pub enum CustomError {
    Exit,
}

impl std::error::Error for CustomError {}

impl BuiltinCommand {
    pub(crate) fn run_with<S, I>(&self, args: I) -> io::Result<ExitStatus>
    where
        I: IntoIterator<Item = S>,
        S: ToString,
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
                    let first = first.to_string();
                    if BuiltinCommand::try_from(first.as_str()).is_ok() {
                        println!("{first} is a shell builtin");
                    } else if let Some(path) = first_match_in_path(first.as_str()) {
                        println!("{} is {}", first, path.display());
                    } else {
                        println!("{first}: not found");
                    }
                } else {
                    unimplemented!()
                }
            }
            BuiltinCommand::Exit => return Err(io::Error::other(CustomError::Exit)),
        }
        Ok(ExitStatus::default())
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
