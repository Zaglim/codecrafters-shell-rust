mod builtin_commands;
mod commands;
mod completion;
mod tokens;

use crate::commands::CommandStream;
use builtin_commands::BuiltinCommand;
use completion::MyCompleter;
use rustyline::{
    config::Configurer, error::ReadlineError, history::FileHistory, CompletionType, Editor, Helper,
};

fn main() -> Result<(), anyhow::Error> {
    #[cfg(debug_assertions)] // logging setup
    init_logging();

    let mut rl = setup_rustyline_editor()?;

    loop {
        let raw_input = match rl.readline("$ ").map(|s| s + "\n") {
            Ok(line) => line,
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => return Ok(()),
            Err(err) => return Err(err.into()),
        };

        #[cfg(debug_assertions)]
        dbg!(&raw_input);

        for command_construction_result in CommandStream::from(&raw_input) {
            #[cfg(debug_assertions)]
            dbg!(&command_construction_result);
            match command_construction_result {
                Ok(command) => {
                    command.run_blocking()?;
                }
                Err(commands::CommandConstructionError::EmptyInput) => println!(),
            }
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

fn init_logging() {
    use log::{max_level, LevelFilter};
    colog::default_builder()
        .filter_level(LevelFilter::Info)
        .init();

    log::log!(
        max_level().to_level().unwrap(),
        "logging level = {}",
        max_level()
    );
}
