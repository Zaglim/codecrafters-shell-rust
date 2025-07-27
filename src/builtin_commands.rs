use crate::executable_path::Executable;
use crate::stream_target::OutStream;
use crate::HISTORY;
use itertools::Itertools;
use my_derives::MyFromStrParse;
use std::fmt::Debug;
use std::io;
use std::io::{Stderr, Stdout, Write};
use std::iter::zip;
use std::str::FromStr;
use std::{
    ffi::OsString, fmt::Display, os::unix::process::ExitStatusExt, path::PathBuf,
    process::ExitStatus,
};
use strum::{EnumIter, IntoStaticStr};

/// "A command that is implemented internally by the shell itself, rather than by an executable program somewhere in the file system."
///
/// -- [ref manual](https://www.gnu.org/software/bash/manual/bash.html#index-builtin-1)
#[derive(Clone, MyFromStrParse, IntoStaticStr, strum::Display, Debug, EnumIter)]
pub enum BuiltinCommand {
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
    #[strum(serialize = "history")]
    History,
}

impl BuiltinCommand {
    pub(crate) fn run_with<S, I>(
        &self,
        args: I,
        mut out_redirect: OutStream<Stdout>,
        mut err_redirect: OutStream<Stderr>,
    ) -> io::Result<ExitStatus>
    where
        I: IntoIterator<Item = S> + Debug,
        S: Display + Debug + AsRef<str>,
    {
        let mut args_iter = args.into_iter();

        match self {
            Self::Exit => std::process::exit(0),
            Self::Echo => {
                writeln!(out_redirect, "{}", args_iter.format(" "))?;
            }
            Self::Type => {
                for arg in args_iter {
                    if arg.as_ref().parse::<Self>().is_ok() {
                        writeln!(out_redirect, "{arg} is a shell builtin")?;
                    } else if let Some(path) = arg.as_ref().first_executable_match_in_path() {
                        writeln!(out_redirect, "{arg} is {}", path.display(),)?;
                    } else {
                        writeln!(out_redirect, "{arg}: not found")?;
                    }
                }
            }
            Self::PrintWorkingDir => {
                let current_dir = std::env::current_dir()?;
                writeln!(out_redirect, "{}", current_dir.to_string_lossy())?;
            }
            Self::ChangeDir => {
                let mut path: PathBuf = args_iter
                    .next()
                    .map_or_else(PathBuf::new, |s| PathBuf::from(s.as_ref()));
                let mut path_components = path.components();
                if path_components.next()
                    == Some(std::path::Component::Normal(&OsString::from("~")))
                {
                    let home: PathBuf = std::env::var("HOME").unwrap().into();
                    path = {
                        let mut builder = home;
                        builder.extend(path_components);
                        builder
                    }
                }

                let cd_result = std::env::set_current_dir(&path);

                if cd_result.is_err() {
                    writeln!(
                        err_redirect,
                        "cd: {}: No such file or directory",
                        &path.to_string_lossy(),
                    )?;
                    return Ok(ExitStatus::from_raw(2));
                }
            }
            Self::History => {
                HISTORY.with_borrow(|vec| {
                    let size = if let Some(value) = args_iter.next() {
                        if let Ok(n) = usize::from_str(value.as_ref()) {
                            n
                        } else {
                            writeln!(err_redirect, "expected a number").unwrap();
                            todo!("return appropriate exit status");
                        }
                    } else {
                        vec.len()
                    };
                    for (num, item) in zip((vec.len() - size + 1)..=size, vec) {
                        writeln!(out_redirect, "{num:>5} {item}").unwrap(); // todo handle write error
                    }
                });
            }
        }
        Ok({
            log::info!("builtin command executed with success");
            ExitStatus::default()
        })
    }
}
