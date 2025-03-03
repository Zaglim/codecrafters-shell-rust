use my_derives::MyFromStrParse;
use strum::{EnumIter, IntoStaticStr};

use std::ffi::OsString;
use std::fmt::Display;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
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
    PrintWorkingDir,
    #[strum(serialize = "cd")]
    ChangeDir,
}

impl BuiltinCommand {
    pub(crate) fn run_with<S, I>(&self, args: I) -> anyhow::Result<ExitStatus>
    where
        I: IntoIterator<Item = S>,
        S: Display,
    {
        use BuiltinCommand as BC;
        let mut arg_strings = args.into_iter().map(|s| s.to_string());
        match self {
            BC::Exit => std::process::exit(0),
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
            BC::PrintWorkingDir => {
                let current_dir = std::env::current_dir()?;

                println!(
                    "{}",
                    String::from_utf8(current_dir.as_os_str().as_bytes().to_vec())?
                );
            }
            BC::ChangeDir => {
                let mut path: PathBuf = arg_strings.next().unwrap_or(String::new()).into();
                let mut path_components = path.components();
                if path_components.next()
                    == Some(std::path::Component::Normal(&OsString::from("~")))
                {
                    let home: PathBuf = std::env::var("HOME").unwrap().into();
                    path = {
                        let mut builder = home.clone();
                        builder.extend(path_components);
                        builder
                    }
                }

                let cd_result = std::env::set_current_dir(&path);

                if cd_result.is_err() {
                    eprintln!("cd: {}: No such file or directory", &path.to_string_lossy());
                    return Ok(ExitStatus::from_raw(2));
                }
            }
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
