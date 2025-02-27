use std::collections::HashSet;

use rustyline::completion::Completer;
use rustyline::{Helper, Highlighter, Hinter, Validator};
use strum::IntoEnumIterator;

use crate::{BuiltinCommand, PATH};

#[derive(Helper, Hinter, Highlighter, Validator)]
pub struct MyCompleter {
    commands: HashSet<String>,
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
                .map(|s| s.to_string())
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
        let mut possible: Vec<_> = self
            .commands
            .iter()
            .filter(|c| c.starts_with(line))
            .map(|s| s.to_string())
            .collect();

        possible.sort();

        Ok((0, possible))
    }

    fn update(
        &self,
        line: &mut rustyline::line_buffer::LineBuffer,
        start: usize,
        elected: &str,
        cl: &mut rustyline::Changeset,
    ) {
        let text = elected.to_string() + " ";
        // todo!(" add space at end");
        let end = line.pos();
        line.replace(start..end, text.as_str(), cl);
    }
}
