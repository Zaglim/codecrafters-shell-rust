use crate::{executable_path::Executable, stream_target::OutStream, tokens::Token, EDITOR};
use itertools::Itertools;
use my_derives::MyFromStrParse;
use rustyline::{error::ReadlineError, history::History};
use std::fs::File;
use std::io::{read_to_string, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
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
                match args_iter.next() {
                    Some(d @ ("-a" | "-w")) => {
                        // write or append history in memory to a file

                        let file_path: Box<Path> = args_iter
                            .next()
                            .map_or_else(history_default_path, |s| Path::new(s).into());

                        let mut editor = EDITOR.write().unwrap();

                        match d {
                            "-a" => editor.append_history(&file_path).unwrap(),
                            "-w" => editor.save_history(&file_path).unwrap(),
                            _ => unreachable!(),
                        }
                        drop(editor);

                        let mut file = File::options().read(true).write(true).open(file_path)?;

                        let mut reader = BufReader::new(&mut file);

                        // remove any starting line "#V2"
                        // todo use costom implementor of `History` to avoid this awkward overrride
                        if reader
                            .by_ref()
                            .lines()
                            .next()
                            .is_some_and(|r| r.is_ok_and(|line| line == "#V2"))
                        {
                            // override file with the file excluding that first line
                            let string = read_to_string(reader)?;
                            file.set_len(0)?;
                            file.seek(SeekFrom::Start(0))?;
                            write!(file, "{string}")?;
                            file.sync_all()?;
                        }

                        Ok(ExitStatus::default())
                    }
                    Some("-r") => {
                        // append loaded history with all the given files

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

pub fn history_default_path() -> Box<Path> {
    const HISTFILE_KEY: &str = "HISTFILE";
    const BACKUP: &str = "~/.bash_history";

    std::env::var(HISTFILE_KEY).map_or_else(
        |_| {
            log::trace!("{HISTFILE_KEY} not declared in current env. Using backup: `{BACKUP}`");
            Path::new(BACKUP).into()
        },
        |h| PathBuf::from(h).into(),
    )
}
