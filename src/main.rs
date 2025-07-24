mod builtin_commands;
mod commands;
mod completion;
mod executable_path;
mod stream_target;
mod tokens;

use crate::commands::{Command, CommandStream};
use builtin_commands::BuiltinCommand;
use completion::MyCompleter;
use rustyline::{
    config::Configurer, error::ReadlineError, history::FileHistory, CompletionType, Editor, Helper,
};

fn main() -> Result<(), anyhow::Error> {
    #[cfg(debug_assertions)] // logging setup
    init_logging();

    let mut rl = setup_rustyline_editor()?;

    'read_line: loop {
        let raw_input_line = match rl.readline("$ ") {
            Ok(line) => line,
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => return Ok(()),
            Err(err) => return Err(err.into()),
        };

        let command_stream = CommandStream::from(&raw_input_line);

        for command_construction_result in command_stream {
            let pipeline = match command_construction_result {
                Err(e) => {
                    log::warn!("received error: {e:?}");
                    eprintln!("{e}");
                    continue 'read_line;
                }
                Ok(command) => command,
            };
            pipeline.run_blocking()?;
        }
    }
}

fn setup_rustyline_editor() -> Result<Editor<impl Helper, FileHistory>, anyhow::Error> {
    let completer = MyCompleter::default();
    let mut rl = Editor::new()?;
    rl.set_helper(Some(completer));
    rl.set_completion_type(CompletionType::List);
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
