#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    

    // Wait for user input
    let stdin = io::stdin();
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        stdin.read_line(&mut input).unwrap();
        println!("{}: command not found", input.trim())
    }
}
