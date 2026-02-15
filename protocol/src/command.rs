use crate::resp::RespFrame;

#[derive(Clone, Debug, PartialEq)] // TODO: to remove
pub enum Value {
    String(String),
    Integer(i64),
    List(Vec<Value>),
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Ping(Option<String>),
    Command,
    Set(String, Value),
    Get(String),
    Del(Vec<String>),
    Echo(String),
}

#[derive(Debug, PartialEq)]
pub enum CommandError {
    NotAnArray,
    EmptyCommand,
    InvalidCommandName,
    UnknownCommand(String),
    WrongArity,
}

fn frame_to_value(frame: RespFrame) -> Result<Value, CommandError> {
    match frame {
        RespFrame::BulkStrings(Some(s)) | RespFrame::SimpleString(s) => Ok(Value::String(s)),
        RespFrame::Integer(i) => Ok(Value::Integer(i)),
        _ => Err(CommandError::WrongArity),
    }
}

fn expect_bulk_string(frame: &RespFrame) -> Result<String, CommandError> {
    match frame {
        RespFrame::BulkStrings(Some(s)) => Ok(s.clone()),
        _ => Err(CommandError::WrongArity),
    }
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
            "PING" => {
                if frames.len() > 2 {
                    return Err(CommandError::WrongArity);
                }
                let msg = if frames.len() == 2 {
                    Some(expect_bulk_string(&frames[1])?)
                } else {
                    None
                };
                Ok(Command::Ping(msg))
            }
            "SET" => {
                if frames.len() != 3 {
                    return Err(CommandError::WrongArity);
                }
                let key = expect_bulk_string(&frames[1])?;
                let value = frame_to_value(frames[2].clone())?;
                Ok(Command::Set(key, value))
            },
            "GET" => {
                if frames.len() != 2 {
                    return Err(CommandError::WrongArity);
                }
                let key = expect_bulk_string(&frames[1])?;
                Ok(Command::Get(key))
            },
            "DEL" => {
                if frames.len() < 2 {
                    return Err(CommandError::WrongArity);
                }
                // DEL takes one or more keys and deletes them. It returns an integer (number of keys deleted)
                let mut keys = Vec::new();
                for frame in &frames[1..] {
                    let key = expect_bulk_string(frame)?;
                    keys.push(key);
                }
                Ok(Command::Del(keys))
            },
            "ECHO" => {
                if frames.len() != 2 {
                    return Err(CommandError::WrongArity);
                }
                let msg = expect_bulk_string(&frames[1])?;
                Ok(Command::Echo(msg))
            },
            "COMMAND" => Ok(Command::Command),
            _ => Err(CommandError::UnknownCommand(name)),
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {                                                                                                             
            CommandError::WrongArity => {                                                                                  
                write!(f, "ERR wrong number of arguments for command")
            },
            CommandError::UnknownCommand(name) => {
                write!(f, "ERR unknown command '{}'", name)
            },
            _ => write!(f, "ERR {:?}", self)
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
        assert_eq!(Command::from_frame(frame), Ok(Command::Ping(None)));
    }

    #[test]
    fn ping_with_message() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("PING".into())),
            RespFrame::BulkStrings(Some("hello".into())),
        ]);
        assert_eq!(Command::from_frame(frame), Ok(Command::Ping(Some("hello".into()))));
    }

    #[test]
    fn ping_too_many_args() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("PING".into())),
            RespFrame::BulkStrings(Some("a".into())),
            RespFrame::BulkStrings(Some("b".into())),
        ]);
        assert_eq!(Command::from_frame(frame), Err(CommandError::WrongArity));
    }

    #[test]
    fn ping_case_insensitive() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("ping".into())),
        ]);
        assert_eq!(Command::from_frame(frame), Ok(Command::Ping(None)));
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

    #[test]
    fn set_string_value() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("SET".into())),
            RespFrame::BulkStrings(Some("key".into())),
            RespFrame::BulkStrings(Some("value".into())),
        ]);
        assert_eq!(
            Command::from_frame(frame),
            Ok(Command::Set("key".into(), Value::String("value".into())))
        );
    }

    #[test]
    fn set_integer_value() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("SET".into())),
            RespFrame::BulkStrings(Some("key".into())),
            RespFrame::Integer(42),
        ]);
        assert_eq!(
            Command::from_frame(frame),
            Ok(Command::Set("key".into(), Value::Integer(42)))
        );
    }

    #[test]
    fn set_case_insensitive() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("set".into())),
            RespFrame::BulkStrings(Some("k".into())),
            RespFrame::BulkStrings(Some("v".into())),
        ]);
        assert_eq!(
            Command::from_frame(frame),
            Ok(Command::Set("k".into(), Value::String("v".into())))
        );
    }

    #[test]
    fn set_wrong_arity_too_few() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("SET".into())),
            RespFrame::BulkStrings(Some("key".into())),
        ]);
        assert_eq!(Command::from_frame(frame), Err(CommandError::WrongArity));
    }

    #[test]
    fn set_wrong_arity_too_many() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("SET".into())),
            RespFrame::BulkStrings(Some("key".into())),
            RespFrame::BulkStrings(Some("val".into())),
            RespFrame::BulkStrings(Some("extra".into())),
        ]);
        assert_eq!(Command::from_frame(frame), Err(CommandError::WrongArity));
    }

    #[test]
    fn del_single_key() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("DEL".into())),
            RespFrame::BulkStrings(Some("mykey".into())),
        ]);
        assert_eq!(
            Command::from_frame(frame),
            Ok(Command::Del(vec!["mykey".into()]))
        );
    }

    #[test]
    fn del_multiple_keys() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("DEL".into())),
            RespFrame::BulkStrings(Some("k1".into())),
            RespFrame::BulkStrings(Some("k2".into())),
            RespFrame::BulkStrings(Some("k3".into())),
        ]);
        assert_eq!(
            Command::from_frame(frame),
            Ok(Command::Del(vec!["k1".into(), "k2".into(), "k3".into()]))
        );
    }

    #[test]
    fn del_no_keys() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("DEL".into())),
        ]);
        assert_eq!(Command::from_frame(frame), Err(CommandError::WrongArity));
    }

    #[test]
    fn del_case_insensitive() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("del".into())),
            RespFrame::BulkStrings(Some("mykey".into())),
        ]);
        assert_eq!(
            Command::from_frame(frame),
            Ok(Command::Del(vec!["mykey".into()]))
        );
    }

    #[test]
    fn echo_basic() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("ECHO".into())),
            RespFrame::BulkStrings(Some("hello".into())),
        ]);
        assert_eq!(Command::from_frame(frame), Ok(Command::Echo("hello".into())));
    }

    #[test]
    fn echo_case_insensitive() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("echo".into())),
            RespFrame::BulkStrings(Some("world".into())),
        ]);
        assert_eq!(Command::from_frame(frame), Ok(Command::Echo("world".into())));
    }

    #[test]
    fn echo_no_args() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("ECHO".into())),
        ]);
        assert_eq!(Command::from_frame(frame), Err(CommandError::WrongArity));
    }

    #[test]
    fn echo_too_many_args() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("ECHO".into())),
            RespFrame::BulkStrings(Some("a".into())),
            RespFrame::BulkStrings(Some("b".into())),
        ]);
        assert_eq!(Command::from_frame(frame), Err(CommandError::WrongArity));
    }
}