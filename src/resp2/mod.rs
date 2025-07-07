pub mod command;
pub mod info;
pub mod serialization;

use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::{Arc, Mutex},
};

use command::*;
use serialization::*;

use crate::{common::Environment, resp2::info::InfoSection};

pub struct Resp2 {
    kind: Resp2Command,
    data: Vec<String>,
    is_array: bool,
    stream: Option<std::net::TcpStream>,
    environment: Arc<Mutex<Environment>>,
}

#[allow(dead_code)]
impl Resp2 {
    pub fn new(environment: Arc<Mutex<Environment>>) -> Self {
        Resp2 {
            kind: Resp2Command::UNDEFINED,
            data: Vec::new(),
            is_array: true,
            stream: None,
            environment,
        }
    }

    pub fn set_kind(&mut self, kind: Resp2Command) {
        self.kind = kind;
    }

    pub fn set_data(&mut self, data: Vec<String>) {
        self.data = data;
    }

    pub fn set_is_array(&mut self, is_array: bool) {
        self.is_array = is_array;
    }

    pub fn set_stream(&mut self, stream: std::net::TcpStream) {
        self.stream = Some(stream);
    }

    pub fn set_environment(&mut self, environment: Arc<Mutex<Environment>>) {
        self.environment = environment;
    }

    fn handle_deserialization(&mut self, input: &str) -> Result<(), String> {
        let mut parts = input.split("\r\n");

        let len_str = parts.next().ok_or("Missing RESP2 array header")?;
        if !len_str.starts_with('*') {
            return Err(format!("Invalid RESP2 array header: '{}'", len_str));
        }

        let expected_len = len_str[1..]
            .parse::<usize>()
            .map_err(|_| format!("Invalid RESP2 array length: '{}'", len_str))?;

        self.data.clear();
        self.data.reserve(expected_len);

        for _ in 0..expected_len {
            let size_str = parts.next().ok_or("Missing $<size> header")?;
            if !size_str.starts_with('$') {
                return Err(format!("Expected $, got '{}'", size_str));
            }

            let size = size_str[1..]
                .parse::<usize>()
                .map_err(|_| format!("Invalid bulk string size: '{}'", size_str))?;

            let value = parts.next().ok_or("Missing bulk string content")?;
            if value.len() != size {
                return Err(format!(
                    "Data length mismatch: expected {}, got '{}'",
                    size, value
                ));
            }

            self.data.push(value.to_string());
        }

        self.kind = self
            .data
            .first()
            .map(|cmd| Resp2Command::from_str(cmd))
            .unwrap_or(Resp2Command::UNDEFINED);

        Ok(())
    }

    pub fn reflect(&mut self) -> Result<(), String> {
        match self.kind {
            Resp2Command::PING => {
                let stream = self.stream.as_mut().ok_or("Missing stream")?;
                stream
                    .write_all(b"+PONG\r\n")
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
            Resp2Command::ECHO => {
                let stream = self.stream.as_mut().ok_or("Missing stream")?;
                let msg = self.data.get(1).cloned().unwrap_or_default();
                let response = format!("+{}\r\n", msg);
                stream
                    .write_all(response.as_bytes())
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
            Resp2Command::SET => {
                let stream = self.stream.as_mut().ok_or("Missing stream")?;

                if self.data.len() < 3 {
                    return Err("SET command requires at least 3 arguments".to_string());
                }
                let key = &self.data[1];
                let value = &self.data[2];

                let mut exp: Option<u64> = None;
                if self.data.len() == 5 && self.data[3].to_uppercase() == "PX" {
                    let ms = &self.data[4];
                    exp = match ms.parse::<u64>() {
                        Ok(val) => Some(val),
                        Err(_) => {
                            return Err(format!("Invalid PX value: '{}'", ms));
                        }
                    };
                }

                self.environment.lock().map_err(|e| e.to_string())?.set(
                    key.clone(),
                    value.clone(),
                    exp,
                );
                stream
                    .write_all(b"+OK\r\n")
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
            Resp2Command::GET => {
                let stream = self.stream.as_mut().ok_or("Missing stream")?;

                if self.data.len() < 2 {
                    return Err("GET command requires at least 2 arguments".to_string());
                }
                let key = &self.data[1];
                let mut env = self.environment.lock().map_err(|e| e.to_string())?;
                match env.get(key) {
                    Some(val) => {
                        let response = format!("+{}\r\n", val);
                        stream
                            .write_all(response.as_bytes())
                            .map_err(|e| format!("Failed to write to stream: {}", e))?;
                    }
                    None => {
                        stream
                            .write_all(b"$-1\r\n")
                            .map_err(|e| format!("Failed to write to stream: {}", e))?;
                    }
                }
            }
            Resp2Command::INFO => {
                let stream = self.stream.as_mut().ok_or("Missing stream")?;

                if self.data.len() < 2 {
                    return Err("INFO command requires at least 1 argument".to_string());
                }
                let section = &self.data[1];
                let response: Vec<u8> = match info::InfoSection::from_str(section) {
                    InfoSection::REPLICATION => {
                        let env = self.environment.lock().map_err(|e| e.to_string())?;
                        let content = format!(
                            "role:{}\r\nmaster_replid:{}\r\nmaster_repl_offset:{}",
                            env.role(),
                            env.master_replid(),
                            env.master_repl_offset()
                        );

                        format!("${}\r\n{}\r\n", content.len(), content)
                            .as_bytes()
                            .to_vec()
                    }
                };

                stream
                    .write_all(&response)
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
            Resp2Command::INTITIALIZE => {
                let mut env = self.environment.lock().map_err(|e| e.to_string())?;
                let master_host = env.master_host().ok_or("Master host not set")?;
                let master_port = env.master_port().ok_or("Master port not set")?;
                let mut master_stream = TcpStream::connect((master_host, master_port))
                    .map_err(|e| format!("Failed to connect to master: {}", e))?;

                // PING
                let mut ping = Resp2::new(self.environment.clone());
                ping.set_kind(Resp2Command::PING);
                ping.set_is_array(true);
                ping.set_data(vec!["PING".to_string()]);
                let ping_payload: Vec<u8> = ping.serialize();
                master_stream
                    .write_all(&ping_payload)
                    .map_err(|e| format!("Failed to send handshake to master: {}", e))?;

                // PONG
                if self.read_master(&mut master_stream).is_err() {
                    return Err("Failed to read from master".to_string());
                }

                // REPLCONF listening-port <PORT>
                let mut replconf = Resp2::new(self.environment.clone());
                replconf.set_kind(Resp2Command::REPLCONF);
                replconf.set_is_array(true);
                replconf.set_data(vec![
                    "REPLCONF".to_string(),
                    "listening-port".to_string(),
                    env.port().to_string(),
                ]);
                let replconf_payload: Vec<u8> = replconf.serialize();
                master_stream
                    .write_all(&replconf_payload)
                    .map_err(|e| format!("Failed to send REPLCONF to master: {}", e))?;

                // OK
                if self.read_master(&mut master_stream).is_err() {
                    return Err("Failed to read from master".to_string());
                }

                // REPLCONF capa psync2
                let mut replconf_capa = Resp2::new(self.environment.clone());
                replconf_capa.set_kind(Resp2Command::REPLCONF);
                replconf_capa.set_is_array(true);
                replconf_capa.set_data(vec![
                    "REPLCONF".to_string(),
                    "capa".to_string(),
                    "psync2".to_string(),
                ]);
                let replconf_capa_payload: Vec<u8> = replconf_capa.serialize();
                master_stream
                    .write_all(&replconf_capa_payload)
                    .map_err(|e| format!("Failed to send REPLCONF capa to master: {}", e))?;

                // OK
                if self.read_master(&mut master_stream).is_err() {
                    return Err("Failed to read from master".to_string());
                }

                // PSYNC <REPLID> <OFFSET>
                let mut psync = Resp2::new(self.environment.clone());
                psync.set_kind(Resp2Command::PSYNC);
                psync.set_is_array(true);
                psync.set_data(vec!["PSYNC".to_string(), "?".to_string(), "-1".to_string()]);

                let psync_payload: Vec<u8> = psync.serialize();
                master_stream
                    .write_all(&psync_payload)
                    .map_err(|e| format!("Failed to send PSYNC to master: {}", e))?;

                // Update master_replid and master_repl_offset
                let mut buffer = vec![0; 1024];
                let n = master_stream
                    .read(&mut buffer)
                    .map_err(|e| format!("Failed to read from stream: {}", e))?;
                if n == 0 {
                    return Err("Connection closed by master".to_string());
                }
                let end = buffer[..n].windows(2).position(|w| w == b"\r\n");
                if let Some(pos) = end {
                    let line = &buffer[..pos];
                    let header = std::str::from_utf8(line)
                        .map_err(|e| format!("Header is not valid UTF-8: {}", e))?;

                    if !header.starts_with("+FULLRESYNC") {
                        return Err(format!("Unexpected response from master: '{}'", header));
                    }

                    let parts: Vec<&str> = header.split_whitespace().collect();
                    if parts.len() < 3 {
                        return Err("Invalid FULLRESYNC response from master".to_string());
                    }
                    let replid = parts[1].to_string();
                    let offset = parts[2]
                        .parse::<u64>()
                        .map_err(|_| "Invalid offset in FULLRESYNC response".to_string())?;

                    env.set_master_replid(replid);
                    env.set_master_repl_offset(offset);
                } else {
                    return Err("No CRLF found in FULLRESYNC response".to_string());
                }
            }
            Resp2Command::REPLCONF => {
                let stream = self.stream.as_mut().ok_or("Missing stream")?;
                stream
                    .write_all(b"+OK\r\n")
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
            Resp2Command::PSYNC => {
                let stream = self.stream.as_mut().ok_or("Missing stream")?;
                // let env = self.environment.lock().map_err(|e| e.to_string())?;
                let response = format!(
                    "+FULLRESYNC {} {}\r\n",
                    "8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb".to_string(),
                    0
                );
                stream
                    .write_all(response.as_bytes())
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
            _ => {
                let stream = self.stream.as_mut().ok_or("Missing stream")?;
                let err = b"-ERR unknown command\r\n";
                stream
                    .write_all(err)
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
        }

        Ok(())
    }

    fn read_master(&self, stream: &mut TcpStream) -> Result<bool, String> {
        let mut buffer = vec![0; 1024];
        let n = stream
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read from stream: {}", e))?;
        if n == 0 {
            return Err("Connection closed by master".to_string());
        }

        let buffer = match String::from_utf8(buffer[..n].to_vec()) {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to convert bytes to string: {}", e)),
        };
        if !buffer.starts_with("+OK") && !buffer.starts_with("+PONG") {
            return Err(format!("Unexpected response from master: '{}'", buffer));
        }

        Ok(true)
    }
}

impl Serialize<String> for Resp2 {
    fn serialize(&self) -> String {
        let mut out = String::new();
        if self.is_array {
            out.push_str(&format!("*{}\r\n", self.data.len()));
        }
        for part in &self.data {
            out.push_str(&format!("${}\r\n{}\r\n", part.len(), part));
        }

        out
    }
}

impl Serialize<Vec<u8>> for Resp2 {
    fn serialize(&self) -> Vec<u8> {
        let mut out = String::new();
        if self.is_array {
            out.push_str(&format!("*{}\r\n", self.data.len()));
        }

        let mut out = out.into_bytes();

        for part in &self.data {
            out.extend_from_slice(format!("${}\r\n", part.len()).as_bytes());
            out.extend_from_slice(part.as_bytes());
            out.push(b'\r');
            out.push(b'\n');
        }

        out
    }
}

impl Deserialize<&str> for Resp2 {
    fn deserialize(&mut self, input: &str) -> Result<(), String> {
        self.handle_deserialization(input)
    }
}

impl Deserialize<Vec<u8>> for Resp2 {
    fn deserialize(&mut self, input: Vec<u8>) -> Result<(), String> {
        let input = String::from_utf8(input).map_err(|e| e.to_string())?;
        self.handle_deserialization(&input)
    }
}
