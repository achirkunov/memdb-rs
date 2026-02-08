// TODO: Handle null arrays (*-1\r\n) — change Arrays(Vec<RespFrame>) to Arrays(Option<Vec<RespFrame>>)
// OR add RespFrame::Null

#[derive(Debug)]
enum ParseError {
    MissingCLRF,
    InvalidUTF8,
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


    fn parse_line(input: &[u8]) -> Result<(&[u8],&[u8]), ParseError> {
        let pos = input.windows(2).position(|w| w == b"\r\n").ok_or(ParseError::MissingCLRF)?;
        Ok((&input[..pos], &input[pos + 2..] ))
    }

    fn parse_bytes(input: &[u8]) -> Result<(RespFrame, &[u8]), ParseError> {
        if input.is_empty() {
            return Err(ParseError::Incomplete);
        }
        let (&first, rest) = input.split_first().unwrap();
        match first {
            b'+' => {
                let (data, rest) = Self::parse_line(rest)?;
                let data = std::str::from_utf8(data).map_err(|_| ParseError::Incomplete )?;
                Ok((RespFrame::SimpleString(data.to_string()), rest))
            },
            b'-' => {
                let (data, rest) = Self::parse_line(rest)?;
                let data = std::str::from_utf8(data).map_err(|_| ParseError::InvalidUTF8 )?;
                Ok((RespFrame::SimpleError(data.to_string()), rest))
            },
            b':' => {
                let (data, rest) = Self::parse_line(rest)?;
                let data = std::str::from_utf8(data).map_err(|_| ParseError::InvalidUTF8 )?.parse::<i64>().map_err(|_| ParseError::InvalidInteger)?;
                Ok((RespFrame::Integer(data), rest))
            },
            b'$' => {
                let (data, rest) = Self::parse_line(rest)?;
                let data = std::str::from_utf8(data).map_err(|_| ParseError::InvalidUTF8 )?;
                // The spec uses a 512 MB defauolt max for bulk strings, so usize is enough
                let len: i64 = data.parse().map_err(|_| ParseError::InvalidInteger )?;
                if len == -1 {
                    return Ok((RespFrame::BulkStrings(None), rest))
                }
                let len = len as usize;
                if rest.len() < len + 2 {
                    return Err(ParseError::LengthMismatch);
                }
                if &rest[len..len+2] != b"\r\n" {
                    return Err(ParseError::LengthMismatch);
                }
                let data = std::str::from_utf8(&rest[..len]).map_err(|_| ParseError::InvalidUTF8 )?;
                let rest = &rest[len + 2..];
                Ok((RespFrame::BulkStrings(Some(data.to_string())), rest))
            },
            b'*' => {
                let (data, mut rest) = Self::parse_line(rest)?;
                let data = std::str::from_utf8(data).map_err(|_| ParseError::InvalidUTF8 )?;
                // The spec uses a 512 MB defauolt max for bulk strings, so usize is enough
                let len: i64 = data.parse().map_err(|_| ParseError::InvalidInteger )?;
                if len == 0 {
                    return Ok(
                        (RespFrame::Arrays(vec![]), rest)
                    );
                }
                let mut items = Vec::with_capacity(len as usize);
                // TODO what if usize is <0 ?
                for _ in 0..len as usize {
                    let (item, new_rest) = Self::parse_bytes(rest)?;
                    items.push(item);
                    rest = new_rest;
                }
                Ok((RespFrame::Arrays(items), rest))
            }
            _ => todo!()
        }
    }

    fn parse(s: &str) -> Result<Self, ParseError> {
        let (frame, rest) = Self::parse_bytes(s.as_bytes())?;
        Ok(frame)
    }

    fn marshal(&self) -> Vec<u8> {
        match self {
            RespFrame::SimpleString(s) => format!("+{}\r\n", s).into_bytes(),
            RespFrame::SimpleError(s) => format!("-{}\r\n", s).into_bytes(),
            RespFrame::Integer(i) => format!(":{}\r\n", i).into_bytes(),
            RespFrame::BulkStrings(s) => {
                match s {
                    Some(s) => format!("${}\r\n{}\r\n", s.len(), s).into_bytes(),
                    None => b"$-1\r\n".to_vec()
                }
            },
            RespFrame::Arrays(vec) => {
                // it should be *{}\r\n and for {} we will iterates
                let mut s = format!("*{}\r\n", vec.len()).into_bytes();
                for i in vec {
                    s.extend(i.marshal());
                }
                s
            }
            _ => todo!()
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
    fn parse_simple_string_empty() {
        let result = RespFrame::parse("+\r\n").unwrap();
        assert!(matches!(result, RespFrame::SimpleString(s) if s == ""));
    }

    #[test]
    fn parse_simple_string_with_spaces() {
        let result = RespFrame::parse("+hello world\r\n").unwrap();
        assert!(matches!(result, RespFrame::SimpleString(s) if s == "hello world"));
    }

    #[test]
    fn parse_simple_string_missing_crlf() {
        let result = RespFrame::parse("+OK");
        assert!(matches!(result, Err(ParseError::MissingCLRF)));
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

    #[test]
    fn parse_array_of_integers() {
        let result = RespFrame::parse("*3\r\n:1\r\n:2\r\n:3\r\n").unwrap();
        assert_eq!(result, RespFrame::Arrays(vec![
            RespFrame::Integer(1),
            RespFrame::Integer(2),
            RespFrame::Integer(3),
        ]));
    }

    #[test]
    fn parse_array_mixed_types() {
        let result = RespFrame::parse("*2\r\n+OK\r\n:42\r\n").unwrap();
        assert_eq!(result, RespFrame::Arrays(vec![
            RespFrame::SimpleString("OK".to_string()),
            RespFrame::Integer(42),
        ]));
    }

    #[test]
    fn parse_array_of_bulk_strings() {
        let result = RespFrame::parse("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n").unwrap();
        assert_eq!(result, RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("hello".to_string())),
            RespFrame::BulkStrings(Some("world".to_string())),
        ]));
    }

    #[test]
    fn parse_array_nested() {
        let result = RespFrame::parse("*2\r\n*2\r\n:1\r\n:2\r\n*2\r\n:3\r\n:4\r\n").unwrap();
        assert_eq!(result, RespFrame::Arrays(vec![
            RespFrame::Arrays(vec![
                RespFrame::Integer(1),
                RespFrame::Integer(2),
            ]),
            RespFrame::Arrays(vec![
                RespFrame::Integer(3),
                RespFrame::Integer(4),
            ]),
        ]));
    }

    #[test]
    fn parse_array_with_null_element() {
        let result = RespFrame::parse("*3\r\n$5\r\nhello\r\n$-1\r\n$5\r\nworld\r\n").unwrap();
        assert_eq!(result, RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("hello".to_string())),
            RespFrame::BulkStrings(None),
            RespFrame::BulkStrings(Some("world".to_string())),
        ]));
    }

    #[test]
    fn parse_array_with_embedded_crlf_bulk_string() {
        let result = RespFrame::parse("*1\r\n$7\r\nhel\r\nlo\r\n").unwrap();
        assert_eq!(result, RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("hel\r\nlo".to_string())),
        ]));
    }

    #[test]
    fn parse_array_single_element() {
        let result = RespFrame::parse("*1\r\n:42\r\n").unwrap();
        assert_eq!(result, RespFrame::Arrays(vec![
            RespFrame::Integer(42),
        ]));
    }

    #[test]
    fn parse_array_incomplete() {
        let result = RespFrame::parse("*2\r\n:1\r\n");
        assert!(result.is_err());
    }

    #[test]
    fn parse_line_splits_on_crlf() {
        let (line, rest) = RespFrame::parse_line(b"OK\r\n").unwrap();
        assert_eq!(line, b"OK");
        assert_eq!(rest, b"");
    }

    #[test]
    fn parse_line_with_remainder() {
        let (line, rest) = RespFrame::parse_line(b"5\r\nhello\r\n").unwrap();
        assert_eq!(line, b"5");
        assert_eq!(rest, b"hello\r\n");
    }

    #[test]
    fn parse_line_missing_crlf() {
        let result = RespFrame::parse_line(b"OK");
        assert!(matches!(result, Err(ParseError::MissingCLRF)));
    }

    #[test]
    fn marshal_simple_string() {
        let frame = RespFrame::SimpleString("OK".to_string());
        assert_eq!(frame.marshal(), b"+OK\r\n");
    }

    #[test]
    fn marshal_simple_error() {
        let frame = RespFrame::SimpleError("ERR unknown command".to_string());
        assert_eq!(frame.marshal(), b"-ERR unknown command\r\n");
    }

    #[test]
    fn marshal_integer() {
        let frame = RespFrame::Integer(42);
        assert_eq!(frame.marshal(), b":42\r\n");
    }

    #[test]
    fn marshal_negative_integer() {
        let frame = RespFrame::Integer(-1);
        assert_eq!(frame.marshal(), b":-1\r\n");
    }

    #[test]
    fn marshal_bulk_string() {
        let frame = RespFrame::BulkStrings(Some("hello".to_string()));
        assert_eq!(frame.marshal(), b"$5\r\nhello\r\n");
    }

    #[test]
    fn marshal_bulk_string_empty() {
        let frame = RespFrame::BulkStrings(Some("".to_string()));
        assert_eq!(frame.marshal(), b"$0\r\n\r\n");
    }

    #[test]
    fn marshal_bulk_string_null() {
        let frame = RespFrame::BulkStrings(None);
        assert_eq!(frame.marshal(), b"$-1\r\n");
    }

    #[test]
    fn marshal_bulk_string_with_embedded_crlf() {
        let frame = RespFrame::BulkStrings(Some("hel\r\nlo".to_string()));
        assert_eq!(frame.marshal(), b"$7\r\nhel\r\nlo\r\n");
    }

    #[test]
    fn marshal_integer_zero() {
        let frame = RespFrame::Integer(0);
        assert_eq!(frame.marshal(), b":0\r\n");
    }

    #[test]
    fn marshal_simple_string_empty() {
        let frame = RespFrame::SimpleString("".to_string());
        assert_eq!(frame.marshal(), b"+\r\n");
    }

    #[test]
    fn roundtrip_simple_string() {
        let frame = RespFrame::SimpleString("OK".to_string());
        let bytes = frame.marshal();
        let parsed = RespFrame::parse(std::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(frame, parsed);
    }

    #[test]
    fn roundtrip_bulk_string() {
        let frame = RespFrame::BulkStrings(Some("hel\r\nlo".to_string()));
        let bytes = frame.marshal();
        let parsed = RespFrame::parse(std::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(frame, parsed);
    }

    #[test]
    fn roundtrip_integer() {
        let frame = RespFrame::Integer(-42);
        let bytes = frame.marshal();
        let parsed = RespFrame::parse(std::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(frame, parsed);
    }

    #[test]
    fn marshal_array_empty() {
        let frame = RespFrame::Arrays(vec![]);
        assert_eq!(frame.marshal(), b"*0\r\n");
    }

    #[test]
    fn marshal_array_of_integers() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::Integer(1),
            RespFrame::Integer(2),
            RespFrame::Integer(3),
        ]);
        assert_eq!(frame.marshal(), b"*3\r\n:1\r\n:2\r\n:3\r\n");
    }

    #[test]
    fn marshal_array_of_bulk_strings() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("hello".to_string())),
            RespFrame::BulkStrings(Some("world".to_string())),
        ]);
        assert_eq!(frame.marshal(), b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n");
    }

    #[test]
    fn marshal_array_mixed_types() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::SimpleString("OK".to_string()),
            RespFrame::Integer(42),
        ]);
        assert_eq!(frame.marshal(), b"*2\r\n+OK\r\n:42\r\n");
    }

    #[test]
    fn marshal_array_nested() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::Arrays(vec![
                RespFrame::Integer(1),
                RespFrame::Integer(2),
            ]),
            RespFrame::Arrays(vec![
                RespFrame::Integer(3),
                RespFrame::Integer(4),
            ]),
        ]);
        assert_eq!(frame.marshal(), b"*2\r\n*2\r\n:1\r\n:2\r\n*2\r\n:3\r\n:4\r\n");
    }

    #[test]
    fn marshal_array_with_null_element() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::BulkStrings(Some("hello".to_string())),
            RespFrame::BulkStrings(None),
            RespFrame::BulkStrings(Some("world".to_string())),
        ]);
        assert_eq!(frame.marshal(), b"*3\r\n$5\r\nhello\r\n$-1\r\n$5\r\nworld\r\n");
    }

    #[test]
    fn marshal_array_single_element() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::Integer(42),
        ]);
        assert_eq!(frame.marshal(), b"*1\r\n:42\r\n");
    }

    #[test]
    fn roundtrip_array() {
        let frame = RespFrame::Arrays(vec![
            RespFrame::SimpleString("OK".to_string()),
            RespFrame::Integer(42),
            RespFrame::BulkStrings(Some("hello".to_string())),
            RespFrame::Arrays(vec![
                RespFrame::Integer(1),
                RespFrame::Integer(2),
            ]),
        ]);
        let bytes = frame.marshal();
        let parsed = RespFrame::parse(std::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(frame, parsed);
    }
}
