use crate::resp::RespFrame;

#[derive(Clone, Debug, PartialEq)] // TODO: to remove
pub enum Value {
    String(String),
    Integer(i64),
    List(Vec<Value>),
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Ping,
    Set(String, Value),
    Get(String),
    Del(String),
}

#[derive(Debug, PartialEq)]
pub enum CommandError {
    NotAnArray,
    EmptyCommand,
    InvalidCommandName,
    UnknownCommand(String),
    WrongArity,
}

pub enum Response {
    Pong,
    Set(bool),
    Get(Option<Value>),
    Del(bool),
    Error(String),
}

impl Command {
    pub fn from_frame(frame: RespFrame) -> Result<Command, CommandError> {
        let frames = match frame {
            RespFrame::Arrays(v) => v,
            _ => return Err(CommandError::NotAnArray),
        };

        if frames.is_empty() {
            return Err(CommandError::EmptyCommand);
        }

        let name = match &frames[0] {
            RespFrame::BulkStrings(Some(s)) => s.to_ascii_uppercase(),
            _ => return Err(CommandError::InvalidCommandName),
        };

        match name.as_str() {
            "PING" => Ok(Command::Ping),
            _ => Err(CommandError::UnknownCommand(name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_from_frame() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("PING".into())),
        ]);
        assert_eq!(Command::from_frame(frame), Ok(Command::Ping));
    }

    #[test]
    fn ping_case_insensitive() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("ping".into())),
        ]);
        assert_eq!(Command::from_frame(frame), Ok(Command::Ping));
    }

    #[test]
    fn not_an_array() {
        let frame = RespFrame::SimpleString("PING".into());
        assert_eq!(Command::from_frame(frame), Err(CommandError::NotAnArray));
    }

    #[test]
    fn empty_array() {
        let frame = RespFrame::Arrays(vec![]);
        assert_eq!(Command::from_frame(frame), Err(CommandError::EmptyCommand));
    }

    #[test]
    fn unknown_command() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("FOOBAR".into())),
        ]);
        assert_eq!(
            Command::from_frame(frame),
            Err(CommandError::UnknownCommand("FOOBAR".into()))
        );
    }

    #[test]
    fn invalid_command_name() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::Integer(42),
        ]);
        assert_eq!(
            Command::from_frame(frame),
            Err(CommandError::InvalidCommandName)
        );
    }
}