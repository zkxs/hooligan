// This file is part of hooligan and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2025 Michael Ripley

//! Serialization and deserialization logic for the LocalPlayerModerations file format

use bstr::ByteSlice;
use std::io;
use std::io::Write;

const HIDE_AVATAR_VALUE: &[u8] = b"004";
const SHOW_AVATAR_VALUE: &[u8] = b"005";

/// pre-generated slice of 64 spaces to be used for padding
static PADDING: &[u8] = &[b' '; 64];

#[derive(PartialEq, Eq, Debug)]
pub struct Line {
    /// UTF-8 encoded key
    key: Box<[u8]>,
    /// integer in the range \[000,999]
    value: Value,
}

impl Line {
    pub const fn new(key: Box<[u8]>, value: Value) -> Self {
        Self { key, value }
    }

    pub fn parse(line: &[u8]) -> Result<Self, ParseError> {
        let first_space = line.find_byte(b' ');
        if let Some(first_space) = first_space {
            // UNWRAP: if a space was found from a left scan, then a space must also be found from a right scan
            let last_space = line.rfind_byte(b' ').unwrap();
            let padding = &line[first_space..=last_space];
            let contiguous_space = padding.iter().all(|char| *char == b' ');
            if contiguous_space {
                let key = &line[..first_space];
                let value = &line[last_space + 1..];
                if value.is_empty() {
                    // either the value was missing or there was some trailing space
                    Err(ParseError::bad_split(line))
                } else {
                    // all seems well
                    let value: Value = Value::parse(value).map_err(|_| ParseError::unknown_value(line))?;
                    let key = key.to_owned().into_boxed_slice();
                    Ok(Self { key, value })
                }
            } else {
                // the padding region was not contiguous. In other words, there were more than two space-delimited fields.
                Err(ParseError::bad_split(line))
            }
        } else {
            // there was no space present
            Err(ParseError::bad_split(line))
        }
    }

    pub fn serialize(&self, writer: &mut impl Write) -> io::Result<usize> {
        let mut written = 0;

        // write key
        written += writer.write(&self.key)?;

        // pad with space out to column 64. Always use a minimum of 1 space.
        let spaces = 64usize.saturating_sub(written).max(1);
        written += writer.write(&PADDING[..spaces])?;

        // write value
        written += writer.write(self.value.serialize())?;

        // write newline
        written += writer.write(b"\r\n")?;

        Ok(written)
    }

    pub fn key(&self) -> &[u8] {
        &self.key
    }

    pub const fn value(&self) -> &Value {
        &self.value
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Value {
    Hide,
    Show,
}

impl Value {
    /// `value` is an integer in the range \[000,999]
    const fn parse(value: &[u8]) -> Result<Self, UnknownValue> {
        match value {
            HIDE_AVATAR_VALUE => Ok(Self::Hide),
            SHOW_AVATAR_VALUE => Ok(Self::Show),
            _ => Err(UnknownValue),
        }
    }

    const fn serialize(&self) -> &[u8] {
        match self {
            Self::Hide => HIDE_AVATAR_VALUE,
            Self::Show => SHOW_AVATAR_VALUE,
        }
    }
}

/// zero-size flag to indicate we got an unknown value when parsing the show/hide number
struct UnknownValue;

#[derive(PartialEq, Eq, Debug)]
pub struct ParseError {
    raw_line: Box<[u8]>,
    error_type: ParseErrorType,
}

impl ParseError {
    fn bad_split(line: &[u8]) -> Self {
        Self {
            raw_line: line.to_owned().into_boxed_slice(),
            error_type: ParseErrorType::BadSplit,
        }
    }

    fn unknown_value(line: &[u8]) -> Self {
        Self {
            raw_line: line.to_owned().into_boxed_slice(),
            error_type: ParseErrorType::UnknownValue,
        }
    }

    pub fn serialize(&self, writer: &mut impl Write) -> io::Result<usize> {
        let mut written = 0;
        written += writer.write(self.raw_line.as_bytes())?;
        written += writer.write(b"\r\n")?;
        Ok(written)
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum ParseErrorType {
    BadSplit,
    UnknownValue,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test a normal line with a hidden user
    #[test]
    fn test_line_hide() {
        let actual = Line::parse(b"usr_6b683acd-31a6-495d-aa46-a73c1349f462                        004").unwrap();
        let expected = Line {
            key: b"usr_6b683acd-31a6-495d-aa46-a73c1349f462".to_vec().into_boxed_slice(),
            value: Value::Hide,
        };
        assert_eq!(actual, expected);

        let mut buf = Vec::new();
        actual.serialize(&mut buf).unwrap();
        let expected = b"usr_6b683acd-31a6-495d-aa46-a73c1349f462                        004\r\n";
        assert_eq!(buf, expected);
    }

    /// Test a normal line with a shown user
    #[test]
    fn test_line_show() {
        let actual = Line::parse(b"usr_6b683acd-31a6-495d-aa46-a73c1349f462                        005").unwrap();
        let expected = Line {
            key: b"usr_6b683acd-31a6-495d-aa46-a73c1349f462".to_vec().into_boxed_slice(),
            value: Value::Show,
        };
        assert_eq!(actual, expected);

        let mut buf = Vec::new();
        actual.serialize(&mut buf).unwrap();
        let expected = b"usr_6b683acd-31a6-495d-aa46-a73c1349f462                        005\r\n";
        assert_eq!(buf, expected);
    }

    /// Test an abnormally long line with 77 char key. It should get 1 space of padding.
    #[test]
    fn test_abnormally_long_line_1() {
        // 77 char long key
        let actual =
            Line::parse(b"usr_6b683acd-31a6-495d-aa46-a73c1349f462-6b683acd-31a6-495d-aa46-a73c1349f462 005").unwrap();
        let expected = Line {
            key: b"usr_6b683acd-31a6-495d-aa46-a73c1349f462-6b683acd-31a6-495d-aa46-a73c1349f462"
                .to_vec()
                .into_boxed_slice(),
            value: Value::Show,
        };
        assert_eq!(actual, expected);

        let mut buf = Vec::new();
        actual.serialize(&mut buf).unwrap();
        let expected = b"usr_6b683acd-31a6-495d-aa46-a73c1349f462-6b683acd-31a6-495d-aa46-a73c1349f462 005\r\n";
        assert_eq!(buf, expected);
    }

    /// Test an abnormally long line with 64 char key. It should get 1 space of padding.
    #[test]
    fn test_abnormally_long_line_2() {
        let actual = Line::parse(b"usr_6b683acd-31a6-495d-aa46-a73c1349f462-6b683acd-31a6-495d-aa46 005").unwrap();
        let expected = Line {
            key: b"usr_6b683acd-31a6-495d-aa46-a73c1349f462-6b683acd-31a6-495d-aa46"
                .to_vec()
                .into_boxed_slice(),
            value: Value::Show,
        };
        assert_eq!(actual, expected);

        let mut buf = Vec::new();
        actual.serialize(&mut buf).unwrap();
        let expected = b"usr_6b683acd-31a6-495d-aa46-a73c1349f462-6b683acd-31a6-495d-aa46 005\r\n";
        assert_eq!(buf, expected);
    }

    /// Test an abnormally long line with 63 char key. It should get 1 space of padding.
    #[test]
    fn test_abnormally_long_line_3() {
        let actual = Line::parse(b"usr_6b683acd-31a6-495d-aa46-a73c1349f462-6b683acd-31a6-495d-aa4 005").unwrap();
        let expected = Line {
            key: b"usr_6b683acd-31a6-495d-aa46-a73c1349f462-6b683acd-31a6-495d-aa4"
                .to_vec()
                .into_boxed_slice(),
            value: Value::Show,
        };
        assert_eq!(actual, expected);

        let mut buf = Vec::new();
        actual.serialize(&mut buf).unwrap();
        let expected = b"usr_6b683acd-31a6-495d-aa46-a73c1349f462-6b683acd-31a6-495d-aa4 005\r\n";
        assert_eq!(buf, expected);
    }

    /// Test a line with an unusual key. These are real IDs that can occur in-game that don't match the modern 40-char UUID pattern.
    #[test]
    fn test_line_weird() {
        let actual = Line::parse(b"2ZaOGztkpc                                                      005").unwrap();
        let expected = Line {
            key: b"2ZaOGztkpc".to_vec().into_boxed_slice(),
            value: Value::Show,
        };
        assert_eq!(actual, expected);

        let mut buf = Vec::new();
        actual.serialize(&mut buf).unwrap();
        let expected = b"2ZaOGztkpc                                                      005\r\n";
        assert_eq!(buf, expected);
    }

    /// Test a line with an undocumented value
    #[test]
    fn test_line_unknown_value() {
        let actual = Line::parse(b"2ZaOGztkpc                                                      009").unwrap_err();
        let expected =
            ParseError::unknown_value(b"2ZaOGztkpc                                                      009");
        assert_eq!(actual, expected);

        let mut buf = Vec::new();
        actual.serialize(&mut buf).unwrap();
        let expected = b"2ZaOGztkpc                                                      009\r\n";
        assert_eq!(buf, expected);
    }

    /// Test a malformed line with no second field
    #[test]
    fn test_line_bad_split_not_enough() {
        let actual = Line::parse(b"2ZaOGztkpc").unwrap_err();
        let expected = ParseError::bad_split(b"2ZaOGztkpc");
        assert_eq!(actual, expected);

        let mut buf = Vec::new();
        actual.serialize(&mut buf).unwrap();
        let expected = b"2ZaOGztkpc\r\n";
        assert_eq!(buf, expected);
    }

    /// Test a malformed line with a third field
    #[test]
    fn test_line_bad_split_too_many() {
        let actual =
            Line::parse(b"2ZaOGztkpc                                                      foo bar").unwrap_err();
        let expected =
            ParseError::bad_split(b"2ZaOGztkpc                                                      foo bar");
        assert_eq!(actual, expected);

        let mut buf = Vec::new();
        actual.serialize(&mut buf).unwrap();
        let expected = b"2ZaOGztkpc                                                      foo bar\r\n";
        assert_eq!(buf, expected);
    }
}
