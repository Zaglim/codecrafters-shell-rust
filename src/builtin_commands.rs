use crate::{executable_path::Executable, stream_target::OutStream, tokens::Token, EDITOR};
use itertools::Itertools;
use my_derives::MyFromStrParse;
use rustyline::{error::ReadlineError, history::History};
use std::fs::File;
use std::{
    ffi::OsString,
    fmt::Debug,
    io::{self, Stderr, Stdout, Write},
    iter::zip,
    os::unix::process::ExitStatusExt,
    path::PathBuf,
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
    pub(crate) fn run_with(
        &self,
        args: &[Token],
        mut out_writer: OutStream<Stdout>,
        mut err_writer: OutStream<Stderr>,
    ) -> io::Result<ExitStatus> {
        let mut args_iter = args.iter().map(AsRef::<str>::as_ref).peekable();

        match self {
            Self::Exit => std::process::exit(0),
            Self::Echo => {
                writeln!(out_writer, "{}", args_iter.format(" "))?;
                Ok(ExitStatus::default())
            }
            Self::Type => {
                for arg in args_iter {
                    if arg.parse::<Self>().is_ok() {
                        writeln!(out_writer, "{arg} is a shell builtin")?;
                    } else if let Some(path) = arg.first_executable_match_in_path() {
                        writeln!(out_writer, "{arg} is {}", path.display())?;
                    } else {
                        writeln!(out_writer, "{arg}: not found")?;
                    }
                }
                Ok(ExitStatus::default())
            }
            Self::PrintWorkingDir => {
                let current_dir = std::env::current_dir()?;
                writeln!(out_writer, "{}", current_dir.to_string_lossy())?;
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
                        err_writer,
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
                    Some("-w") => {
                        // write history to a file
                        let file_str: Box<str> = if let Some(arg) = args_iter.next() {
                            arg.into()
                        } else if let Ok(histfile) = std::env::var("HISTFILE") {
                            histfile.into_boxed_str()
                        } else {
                            Box::from("~/.bash_history")
                        };
                        let editor = EDITOR.read().unwrap();

                        let history = editor.history();
                        let mut file = File::create(&*file_str)?;

                        for entry in history {
                            writeln!(file, "{entry}")?;
                        }
                        drop(editor);

                        Ok(ExitStatus::default())
                    }
                    Some("-r") => {
                        // append history with all the given files

                        if args_iter.peek().is_none() {
                            writeln!(err_writer, "expected argument").unwrap();
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
                            err_writer,
                            "history: {not_a_number}: numeric argument required",
                        )
                        .unwrap();
                        todo!("return appropriate exit status");
                    }
                    opt_number_string @ (None | Some(_)) => {
                        // print segment of history

                        let e = EDITOR.read().unwrap();
                        let history = e.history();
                        let size = opt_number_string.map_or(history.len(), |s| {
                            s.parse::<usize>().unwrap_or_else(|_| history.len())
                        });

                        let first_number = history.len().saturating_sub(size) + 1;
                        for (num, item) in zip(first_number.., history.iter().tail(size)) {
                            writeln!(out_writer, "{num:>5} {item}").unwrap(); // todo handle write error
                        }
                        drop(e);
                        Ok(ExitStatus::default())
                    }
                }
            }
        }
    }
}
