use rustyline::completion::Completer;
use rustyline::{Helper, Highlighter, Hinter, Validator};
use strum::IntoEnumIterator;

use crate::{BuiltinCommand, PATH};

#[derive(Helper, Hinter, Highlighter, Validator)]
pub struct MyCompleter {
    commands: Vec<Box<str>>,
}

impl MyCompleter {
    pub fn default() -> Self {
        let path_executables = get_path_executables();
        #[cfg(debug_assertions)]
        dbg!(&path_executables);

        Self {
            commands: BuiltinCommand::iter()
                .map(|s| s.to_string().into_boxed_str())
                .chain(path_executables)
                .collect(),
        }
    }
}

fn get_path_executables() -> Vec<Box<str>> {
    std::env::split_paths(&PATH.to_string())
        .filter_map(|path| {
            Some(
                path.read_dir()
                    .ok()?
                    .filter_map(|entry| {
                        let name = entry.ok()?.file_name().into_string().ok()?.into_boxed_str();
                        Some(name)
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .flatten()
        .collect()
}

impl Completer for MyCompleter {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        _ = (pos, ctx);
        let possible: Vec<_> = self
            .commands
            .iter()
            .filter(|c| c.starts_with(line))
            .map(|s| s.to_string() + " ")
            .collect();

        Ok((0, possible))
    }
}
