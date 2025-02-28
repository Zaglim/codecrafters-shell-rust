use std::collections::HashSet;

use rustyline::completion::Completer;
use rustyline::{Helper, Highlighter, Hinter, Validator};
use strum::IntoEnumIterator;

use crate::BuiltinCommand;

#[derive(Helper, Hinter, Highlighter, Validator)]
pub struct MyCompleter {
    commands: HashSet<String>,
}

impl MyCompleter {
    pub fn default() -> Self {
        let path_executables = get_path_executables();

        Self {
            commands: BuiltinCommand::iter()
                .map(|s| s.to_string())
                .chain(path_executables)
                .collect(),
        }
    }
}

fn get_path_executables() -> Box<[String]> {
    std::env::split_paths(&std::env::var("PATH").unwrap())
        .filter_map(|path| {
            Some(
                path.read_dir()
                    .ok()?
                    .filter_map(|entry| entry.ok()?.file_name().into_string().ok()),
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
        let mut candidates: Vec<_> = self
            .commands
            .iter()
            .filter(|c| c.starts_with(line))
            .map(ToString::to_string)
            .collect();

        candidates.sort();

        if candidates.len() == 1 {
            candidates[0].push(' '); // add a space because the word is completed
        }

        Ok((0, candidates))
    }

    fn update(
        &self,
        line: &mut rustyline::line_buffer::LineBuffer,
        start: usize,
        elected: &str,
        cl: &mut rustyline::Changeset,
    ) {
        let text = elected.to_string();
        let end = line.pos();
        line.replace(start..end, text.as_str(), cl);
    }
}
