use once_cell::unsync::Lazy;
use rustyline::completion::Completer;
use rustyline::{Helper, Highlighter, Hinter, Validator};
use strum::IntoEnumIterator;

use crate::BuiltinCommand;

#[derive(Helper, Hinter, Highlighter, Validator)]
pub struct MyCompleter {
    commands: Vec<&'static str>,
}

impl MyCompleter {
    pub fn default() -> Self {
        Self {
            commands: BuiltinCommand::iter().map(|s| s.into()).collect(),
        }
    }
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

thread_local! {pub static COMPLETER: Lazy<MyCompleter> = Lazy::new(|| MyCompleter {
    commands: BuiltinCommand::iter().map(|s| s.into()).collect(),
});}
