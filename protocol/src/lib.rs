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
    InvalidInteger,
    Incomplete,
    LengthMismatch,
}

#[derive(Debug, PartialEq)]
enum RespFrame {
    SimpleString(String), // +
    SimpleError(String), // -
    Integer(i64), // :
    BulkStrings(Option<String>), // $
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
            b':' => Ok(RespFrame::Integer(value.parse().map_err(|_| ParseError::InvalidInteger)?)),
            b'$' => {
                if value == "-1" {
                    return Ok(RespFrame::BulkStrings(None));
                }
                let (len, data) = value.split_once("\r\n").ok_or(ParseError::Incomplete)?;
                let len: usize = len.parse().map_err(|_| ParseError::InvalidInteger)?;
                if data.len() != len {
                    return Err(ParseError::LengthMismatch);
                }
                Ok(RespFrame::BulkStrings(Some(data.to_string())))
            },
            b'*' => {
                if value == "0" {
                    return Ok(RespFrame::Arrays(vec![]));
                }
                let (len, data) = value.split_once("\r\n").ok_or(ParseError::Incomplete)?;
                let _len: usize = len.parse().map_err(|_| ParseError::InvalidInteger)?;

                todo!("non-empty arrays")
            },
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

    #[test]
    fn parse_integer() {
        let result = RespFrame::parse(":42\r\n").unwrap();
        assert!(matches!(result, RespFrame::Integer(42)));
    }

    #[test]
    fn parse_negative_integer() {
        let result = RespFrame::parse(":-1\r\n").unwrap();
        assert!(matches!(result, RespFrame::Integer(-1)));
    }

    #[test]
    fn parse_invalid_integer() {
        let result = RespFrame::parse(":abc\r\n");
        assert!(matches!(result, Err(ParseError::InvalidInteger)));
    }

    #[test]
    fn parse_large_integer() {
        let result = RespFrame::parse(":2147483648\r\n").unwrap(); // valid i64, exceeds i32
        assert!(matches!(result, RespFrame::Integer(2147483648)));
    }

    #[test]
    fn parse_bulk_string() {
        let result = RespFrame::parse("$5\r\nhello\r\n").unwrap();
        assert!(matches!(result, RespFrame::BulkStrings(Some(s)) if s == "hello"));
    }

    #[test]
    fn parse_bulk_string_empty() {
        let result = RespFrame::parse("$0\r\n\r\n").unwrap();
        assert!(matches!(result, RespFrame::BulkStrings(Some(s)) if s == ""));
    }

    #[test]
    fn parse_bulk_string_length_mismatch() {
        let result = RespFrame::parse("$3\r\nhello\r\n");
        assert!(matches!(result, Err(ParseError::LengthMismatch)));
    }

    #[test]
    fn parse_bulk_string_with_embedded_crlf() {
        let result = RespFrame::parse("$7\r\nhel\r\nlo\r\n").unwrap();
        assert!(matches!(result, RespFrame::BulkStrings(Some(s)) if s == "hel\r\nlo"));
    }

    #[test]
    fn parse_bulk_string_null() {
        let result = RespFrame::parse("$-1\r\n").unwrap();
        assert!(matches!(result, RespFrame::BulkStrings(None)));
    }

    #[test]
    fn parse_array_empty() {
        let result = RespFrame::parse("*0\r\n").unwrap();
        let empty_v = vec![];
        assert_eq!(result, RespFrame::Arrays(empty_v));
    }
}
