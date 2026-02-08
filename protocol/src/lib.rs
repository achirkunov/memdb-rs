//   Simple string — prefixed with +, single line, no newlines allowed:                                                               
//   +OK\r\n         
                                                                                                                                   
//   Bulk string — prefixed with $, includes a length, can contain anything (binary safe):                                            
//   $5\r\n
//   hello\r\n
//   Key differences:
//   - Simple strings can't contain \r\n — bulk strings can (length-delimited, not delimiter-delimited)
//   - Simple strings are for short status replies (+OK, +PONG)
//   - Bulk strings are for arbitrary data (user values, keys, command args)
//   - Bulk strings can represent null: $-1\r\n


#[derive(Debug)]
enum ParseError {
    InvalidPrefix,
    Incomplete,
}

enum RespFrame {
    SimpleString(String), // +
    SimpleError(String), // -
    Integer(i32), // :
    BulkStrings(String), // $
    Arrays(Vec<RespFrame>), // *
}

impl RespFrame {
    // TODO: Data coming off a TCP socket is raw bytes. It might not be valid UTF-8. Change &str to &[u8]
    fn parse(s: &str) -> Result<Self, ParseError> {
        // Every RESP frame ends with \r\n. By stripping it first, the rest of the parsing doesn't have to worry about it
        //let s = s.strip_suffix("\r\n").ok_or(ParseError::Incomplete)?;
        let s = match s.strip_suffix("\r\n") {
            Some(s) => s,
            None => return Err(ParseError::Incomplete),
        };

        let prefix = s.as_bytes()[0];
        let value = &s[1..];

        match prefix {
            b'+' => Ok(RespFrame::SimpleString(value.to_string())),
            b'-' => Ok(RespFrame::SimpleError(value.to_string())),
            _ => Err(ParseError::InvalidPrefix)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_string() {
        let result = RespFrame::parse("+OK\r\n").unwrap();
        assert!(matches!(result, RespFrame::SimpleString(s) if s == "OK"));
    }

    #[test]
    fn parse_simple_error() {
        let result = RespFrame::parse("-ERR unknown command\r\n").unwrap();
        assert!(matches!(result, RespFrame::SimpleError(s) if s == "ERR unknown command"));
    }
}
