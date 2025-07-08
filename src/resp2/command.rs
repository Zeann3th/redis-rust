use std::fmt::Display;

pub enum RespCommand {
    PING,
    UNDEFINED,
    PONG,
    ECHO,
    SET,
    GET,
    INFO,
    INTITIALIZE,
    REPLCONF,
    PSYNC,
}

impl RespCommand {
    pub fn from_str(cmd: &str) -> Self {
        match cmd.to_uppercase().as_str() {
            "PING" => RespCommand::PING,
            "PONG" => RespCommand::PONG,
            "ECHO" => RespCommand::ECHO,
            "SET" => RespCommand::SET,
            "GET" => RespCommand::GET,
            "INFO" => RespCommand::INFO,
            "INTITIALIZE" => RespCommand::INTITIALIZE,
            "REPLCONF" => RespCommand::REPLCONF,
            "PSYNC" => RespCommand::PSYNC,
            _ => RespCommand::UNDEFINED,
        }
    }
}

impl Display for RespCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RespCommand::PING => write!(f, "PING"),
            RespCommand::UNDEFINED => write!(f, "UNDEFINED"),
            RespCommand::PONG => write!(f, "PONG"),
            RespCommand::ECHO => write!(f, "ECHO"),
            RespCommand::SET => write!(f, "SET"),
            RespCommand::GET => write!(f, "GET"),
            RespCommand::INFO => write!(f, "INFO"),
            RespCommand::INTITIALIZE => write!(f, "INTITIALIZE"),
            RespCommand::REPLCONF => write!(f, "REPLCONF"),
            RespCommand::PSYNC => write!(f, "PSYNC"),
        }
    }
}
