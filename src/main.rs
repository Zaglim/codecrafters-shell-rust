#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    let stdin = io::stdin();

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut input = String::new();

        // Wait for user input
        stdin.read_line(&mut input).unwrap();
        match &input.trim()[..] {
            "exit 0" => break,
            _ => println!("{}: command not found", input.trim()),
        }
    }
}
