#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    let stdin = io::stdin();

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut buff = String::new();
        let input = {
            // Wait for user input
            stdin.read_line(&mut buff).unwrap();
            &buff.trim()[..]
        };
        
        let (command, following) = input.split_once(' ').unwrap_or((input, ""));
        
        match command {
            "exit" => { 
                match following {
                    "0" => break,
                    _ => unimplemented!()
                }
            },
            "echo" => println!("{}", following),
            _ => println!("{}: command not found", command),
        }
    }
}
