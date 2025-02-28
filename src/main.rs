mod builtin_commands;
mod commands;
mod completion;
mod tokens;

use crate::commands::CommandStream;
use builtin_commands::{BuiltinCommand, CustomError};
use completion::MyCompleter;
use rustyline::{config::Configurer, error::ReadlineError, CompletionType};

fn main() -> Result<(), anyhow::Error> {
    #[cfg(debug_assertions)] // logging setup
    {
        use log::{max_level, LevelFilter};
        colog::default_builder()
            .filter_level(LevelFilter::Info)
            .init();

        log::log!(
            log::max_level().to_level().unwrap(),
            "logging level = {}",
            max_level()
        );
    }
    let completer = MyCompleter::default();
    let mut rl = rustyline::Editor::new()?;
    rl.set_helper(Some(completer));
    rl.set_completion_type(CompletionType::List);

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
                    match command.run_blocking() {
                        Ok(status) => status,
                        Err(e) if e.is::<CustomError>() => return Ok(()),
                        Err(err) => return Err(err),
                    };
                }
                Err(commands::CommandConstructionError::NoCommand) => println!(),
            }
        }
    }
}
