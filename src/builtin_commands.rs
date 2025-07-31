use crate::executable_path::Executable;
use crate::stream_target::OutStream;
use crate::tokens::Token;
use crate::EDITOR;
use itertools::Itertools;
use my_derives::MyFromStrParse;
use rustyline::error::ReadlineError;
use rustyline::history::History;
use std::fmt::Debug;
use std::io;
use std::io::{Stderr, Stdout, Write};
use std::iter::zip;
use std::{ffi::OsString, os::unix::process::ExitStatusExt, path::PathBuf, process::ExitStatus};
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
    pub(crate) fn run_with(
        &self,
        args: &[Token],
        mut out_redirect: OutStream<Stdout>,
        mut err_redirect: OutStream<Stderr>,
    ) -> io::Result<ExitStatus> {
        let mut args_iter = args.iter().map(AsRef::<str>::as_ref).peekable();

        match self {
            Self::Exit => std::process::exit(0),
            Self::Echo => {
                writeln!(out_redirect, "{}", args_iter.format(" "))?;
                Ok(ExitStatus::default())
            }
            Self::Type => {
                for arg in args_iter {
                    if arg.parse::<Self>().is_ok() {
                        writeln!(out_redirect, "{arg} is a shell builtin")?;
                    } else if let Some(path) = arg.first_executable_match_in_path() {
                        writeln!(out_redirect, "{arg} is {}", path.display())?;
                    } else {
                        writeln!(out_redirect, "{arg}: not found")?;
                    }
                }
                Ok(ExitStatus::default())
            }
            Self::PrintWorkingDir => {
                let current_dir = std::env::current_dir()?;
                writeln!(out_redirect, "{}", current_dir.to_string_lossy())?;
                Ok(ExitStatus::default())
            }
            Self::ChangeDir => {
                let mut path: PathBuf = args_iter.next().map_or_else(PathBuf::new, PathBuf::from);
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
                    Ok(ExitStatus::from_raw(2))
                } else {
                    Ok(ExitStatus::default())
                }
            }
            Self::History => {
                log::debug!("exec History");

                match args_iter.next() {
                    Some("-r") => {
                        // append history with all the given files

                        if args_iter.peek().is_none() {
                            writeln!(err_redirect, "expected argument").unwrap();
                            todo!("return appropriate exit status");
                        }
                        let mut editor = EDITOR.write().unwrap();

                        for arg in args_iter {
                            if arg.starts_with('-') {
                                unimplemented!("more options are not supported yet")
                            }
                            editor.load_history(arg).map_err(|e| match e {
                                ReadlineError::Io(io_e) => io_e,
                                e => unimplemented!("not handling non-io error {e:?}"),
                            })?;
                        }
                        drop(editor);

                        Ok(ExitStatus::default())
                    }
                    Some(not_a_number) if not_a_number.parse::<isize>().is_err() => {
                        // invalid input

                        writeln!(
                            err_redirect,
                            "history: {not_a_number}: numeric argument required",
                        )
                        .unwrap();
                        todo!("return appropriate exit status");
                    }
                    opt_number_string @ (None | Some(_)) => {
                        // print some tail of the history

                        let e = EDITOR.read().unwrap();
                        let history = e.history();
                        let size = opt_number_string.map_or(history.len(), |s| {
                            s.parse::<usize>().unwrap_or_else(|_| history.len())
                        });

                        let first_number = history.len().saturating_sub(size) + 1;
                        for (num, item) in zip(first_number.., history.iter().tail(size)) {
                            writeln!(out_redirect, "{num:>5} {item}").unwrap(); // todo handle write error
                        }
                        drop(e);
                        Ok(ExitStatus::default())
                    }
                }
            }
        }
    }
}
