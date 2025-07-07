use std::{
    io::{Read, Write},
    net::TcpListener,
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buf = String::new();

                match stream.read_to_string(&mut buf) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Failed to read from stream: {}", e);
                        continue;
                    }
                };

                let mut buf = buf.split(" ");

                match buf.next() {
                    Some(command) if command.to_uppercase() == "PING" => match buf.next() {
                        Some(arg) => {
                            match stream.write_all(format!("{}\r\n", arg).as_bytes()) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Failed to write to stream: {}", e);
                                    continue;
                                }
                            };
                        }
                        None => {
                            match stream.write_all(b"PONG\r\n") {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Failed to write to stream: {}", e);
                                    continue;
                                }
                            };
                        }
                    },
                    _ => {
                        println!(
                            "Received unknown command: {}",
                            buf.collect::<Vec<&str>>().join(" ")
                        );
                        continue;
                    }
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
