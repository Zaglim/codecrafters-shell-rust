use my_derives::MyFromStrParse;
use strum::{Display, EnumIter, IntoStaticStr};

use std::fmt::Display;
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
    #[strum(serialize = "pwd")]
    Pwd,
}

#[derive(Debug, Display)]
//
pub enum CustomError {
    Exit,
    StringConversionError,
}

impl std::error::Error for CustomError {}

impl BuiltinCommand {
    pub(crate) fn run_with<S, I>(&self, args: I) -> anyhow::Result<ExitStatus>
    where
        I: IntoIterator<Item = S>,
        S: Display,
    {
        use BuiltinCommand as BC;
        let arg_strings = args.into_iter().map(|s| s.to_string());
        match self {
            BC::Echo => println!("{}", arg_strings.collect::<Vec<_>>().join(" ")),
            BC::Type => {
                for arg in arg_strings {
                    if arg.parse::<BuiltinCommand>().is_ok() {
                        println!("{arg} is a shell builtin");
                    } else if let Some(path) = first_match_in_path(arg.as_str()) {
                        println!("{arg} is {}", path.display());
                    } else {
                        println!("{arg}: not found");
                    }
                }
            }
            BC::Pwd => {
                println!(
                    "{}",
                    std::env::current_dir()?
                        .to_str()
                        .ok_or(CustomError::StringConversionError)?
                );
            }
            BC::Exit => return Err(CustomError::Exit.into()),
        }
        Ok(ExitStatus::default())
    }
}

fn first_match_in_path(name: &str) -> Option<Box<Path>> {
    for path_str in std::env::var("PATH").unwrap().split(':') {
        let path_buf = Path::new(path_str).join(name);
        if path_buf.is_file() {
            return Some(Box::from(path_buf));
        }
    }
    None
}
