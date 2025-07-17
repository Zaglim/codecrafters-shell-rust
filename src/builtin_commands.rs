use crate::executable_path::Executable;
use my_derives::MyFromStrParse;
use std::fmt::{Arguments, Debug};
use std::fs::File;
use std::io::{Read, Write};
use std::{fs, io};
use strum::{EnumIter, IntoStaticStr};

use itertools::Itertools;
use std::{
    ffi::OsString, fmt::Display, os::unix::process::ExitStatusExt, path::PathBuf,
    process::ExitStatus,
};

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
    pub(crate) fn run_with<S, I>(
        &self,
        args: I,
        mut out_redirect: Option<File>,
        mut err_redirect: Option<File>,
    ) -> io::Result<ExitStatus>
    where
        I: IntoIterator<Item = S> + Debug,
        S: Display + Debug + AsRef<str>,
    {
        use BuiltinCommand as BC;

        let mut args_iter = args.into_iter();

        match self {
            BC::Exit => std::process::exit(0),
            BC::Echo => {
                write_stdout(&mut out_redirect, format_args!("{}", args_iter.format(" ")))?;
            }
            BC::Type => {
                for arg in args_iter {
                    if arg.as_ref().parse::<BuiltinCommand>().is_ok() {
                        write_stdout(&mut out_redirect, format_args!("{arg} is a shell builtin"))?;
                    } else if let Some(path) = arg.as_ref().first_executable_match_in_path() {
                        write_stdout(
                            &mut out_redirect,
                            format_args!("{arg} is {}", path.display()),
                        )?;
                    } else {
                        write_stdout(&mut out_redirect, format_args!("{arg}: not found"))?;
                    }
                }
            }
            BC::PrintWorkingDir => {
                let current_dir = std::env::current_dir()?;

                write_stdout(
                    &mut out_redirect,
                    format_args!("{}", current_dir.to_string_lossy()),
                )?;
            }
            BC::ChangeDir => {
                let mut path: PathBuf = args_iter
                    .next()
                    .map_or(PathBuf::new(), |s| PathBuf::from(s.as_ref()));
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
                    write_stderr(
                        &mut err_redirect,
                        format_args!("cd: {}: No such file or directory", &path.to_string_lossy()),
                    )?;
                    return Ok(ExitStatus::from_raw(2));
                }
            }
        }

        Ok({
            log::info!("builtin command executed with success");
            ExitStatus::default()
        })
    }
}

fn write_stdout(redirection: &mut Option<File>, content: Arguments) -> io::Result<()> {
    match redirection {
        Some(ref mut file) => {
            log::info!("writing  to {file:?}");
            writeln!(file, "{content}")?;
            file.flush()?;
            log::info!("successfully wrote to file. File now contains:{:?}", {
                let mut s = String::new();
                file.read_to_string(&mut s).unwrap();
                dbg!(s)
            });
        }
        None => println!("{content}"),
    }
    Ok(())
}

fn write_stderr(redirection: &mut Option<File>, content: Arguments) -> io::Result<()> {
    match redirection {
        Some(ref mut file) => writeln!(file, "{content}")?,
        None => eprintln!("{content}"),
    }
    Ok(())
}
