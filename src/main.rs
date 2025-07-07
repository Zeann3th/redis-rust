#![allow(unused_imports)]
use std::{io::Write, net::TcpListener};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                match stream.write_all(b"+PONG\r\n") {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Failed to write to stream: {}", e);
                        continue;
                    }
                };
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
