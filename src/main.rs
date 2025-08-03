mod builtin_commands;
mod commands;
mod completion;
mod executable_path;
mod stream_target;
mod tokens;

use crate::builtin_commands::history_default_path;
use crate::commands::{Command, CommandStream};
use anyhow::Result as AnyResult;
use builtin_commands::BuiltinCommand;
use completion::MyCompleter;
use rustyline::{
    config::Configurer, error::ReadlineError, history::FileHistory, CompletionType, Config, Editor,
};
use std::sync::{LazyLock, RwLock};

pub static EDITOR: LazyLock<RwLock<Editor<MyCompleter, FileHistory>>> =
    LazyLock::new(|| setup_rustyline_editor().unwrap().into());

fn main() -> AnyResult<()> {
    #[cfg(debug_assertions)] // logging setup
    init_logging();

    loop {
        let raw_line = readline_adding_history()?;

        let command_stream = CommandStream::from(&raw_line);

        for command_construction_result in command_stream {
            let pipeline = match command_construction_result {
                Err(e) => {
                    log::warn!("received error: {e:?}");
                    eprintln!("{e}");
                    break;
                }
                Ok(command) => command,
            };
            pipeline.run_blocking()?;
        }
    }
}

fn readline_adding_history() -> AnyResult<String> {
    let mut editor = EDITOR.write().unwrap();
    let raw_line = match editor.readline("$ ") {
        Ok(line) => line,
        Err(ReadlineError::Interrupted | ReadlineError::Eof) => todo!(),
        Err(err) => return Err(err.into()),
    };
    editor.add_history_entry(&raw_line)?;
    drop(editor);
    Ok(raw_line)
}

fn setup_rustyline_editor() -> Result<Editor<MyCompleter, FileHistory>, anyhow::Error> {
    let completer = MyCompleter::default();
    let mut rl = Editor::new()?;
    rl.set_helper(Some(completer));
    rl.set_completion_type(CompletionType::List);
    rl.load_history(&history_default_path())?;
    Ok(rl)
}

#[cfg(debug_assertions)]
fn init_logging() {
    use log::{max_level, LevelFilter};
    colog::default_builder()
        .filter_module("rustyline", LevelFilter::Off)
        .filter_level(LevelFilter::Trace)
        .init();

    log::log!(
        max_level().to_level().unwrap(),
        "logging level = {}",
        max_level()
    );
}
