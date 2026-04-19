use ruin_rs::run_program;
use std::fs;
use std::io::{self, Read};

fn main() {
    let source = match std::env::args().nth(1) {
        Some(path) => fs::read_to_string(path).expect("Failed to read source file"),
        None => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .expect("Failed to read from stdin");
            buffer
        }
    };

    match run_program(&source) {
        Ok(()) => {}
        Err(error) => eprintln!("Error: {}", error),
    }
}
