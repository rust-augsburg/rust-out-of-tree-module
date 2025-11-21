use std::fs;
use std::io::{Read, Write};

const PATH: &str = "/dev/rust-misc-device";

fn main() {
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(PATH)
        .unwrap();

    // Write to file
    file.write_all(b"test").unwrap();

    // Read from file
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();

    println!("Content: '{data}'")
}
