mod builtin_commands;
mod commands;
mod completion;
mod tokens;

use crate::commands::CommandStream;
use crate::tokens::*;
use builtin_commands::{BuiltinCommand, CustomError};
use completion::MyCompleter;
use once_cell::sync::Lazy;
use rustyline::{config::Configurer, error::ReadlineError, CompletionType};

pub static PATH: Lazy<String> = Lazy::new(|| std::env::var("PATH").unwrap());

fn main() -> Result<(), anyhow::Error> {
    #[cfg(debug_assertions)] // logging setup
    {
        use log::{max_level, LevelFilter};
        colog::default_builder()
            .filter_level(LevelFilter::Warn)
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
                    // if command.is_exit() {
                    //     return Ok(());
                    // }

                    _ = match command.run_blocking() {
                        Ok(status) => status,
                        Err(err) => {
                            // Get the downcasted error type
                            let downcast = err.downcast::<CustomError>();

                            return match downcast {
                                Ok(CustomError::Exit) => Ok(()),
                                Err(err) => Err(err.into()),
                            };
                        }
                    };
                }
                Err(commands::CommandConstructionError::NoCommand) => println!(),
            }
        }
    }
}
