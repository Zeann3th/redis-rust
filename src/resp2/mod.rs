pub mod command;
pub mod serialization;

use std::io::Write;

use command::*;
use serialization::*;

pub struct Resp2 {
    pub kind: Resp2Command,
    pub data: Vec<String>,
    pub stream: Option<std::net::TcpStream>,
}

impl Resp2 {
    pub fn with_stream(stream: std::net::TcpStream) -> Self {
        Resp2 {
            kind: Resp2Command::UNDEFINED,
            data: Vec::new(),
            stream: Some(stream),
        }
    }

    pub fn process_deserialization(&mut self, input: &str) -> Result<(), String> {
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
        let stream = self.stream.as_mut().ok_or("Missing stream")?;

        match self.kind {
            Resp2Command::PING => {
                stream
                    .write_all(b"+PONG\r\n")
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
            Resp2Command::ECHO => {
                let msg = self.data.get(1).cloned().unwrap_or_default();
                let response = format!("+{}\r\n", msg);
                stream
                    .write_all(response.as_bytes())
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
            _ => {
                let err = b"-ERR unknown command\r\n";
                stream
                    .write_all(err)
                    .map_err(|e| format!("Failed to write to stream: {}", e))?;
            }
        }

        Ok(())
    }
}

impl Serialize<String> for Resp2 {
    fn serialize(&self) -> String {
        let mut out = format!("*{}\r\n", self.data.len());

        for part in &self.data {
            out.push_str(&format!("${}\r\n{}\r\n", part.len(), part));
        }

        out
    }
}

impl Serialize<Vec<u8>> for Resp2 {
    fn serialize(&self) -> Vec<u8> {
        let mut out = format!("*{}\r\n", self.data.len()).into_bytes();

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
        self.process_deserialization(input)
    }
}

impl Deserialize<Vec<u8>> for Resp2 {
    fn deserialize(&mut self, input: Vec<u8>) -> Result<(), String> {
        let input = String::from_utf8(input).map_err(|e| e.to_string())?;
        self.process_deserialization(&input)
    }
}
