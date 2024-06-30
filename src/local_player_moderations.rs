// This file is part of hooligan and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2024 Michael Ripley

//! Serialization and deserialization logic for the LocalPlayerModerations file format

const HIDE_AVATAR_VALUE: &str = "004";
const SHOW_AVATAR_VALUE: &str = "005";

#[derive(PartialEq, Eq, Debug)]
pub struct Line {
    /// UTF-8 encoded key
    pub key: String,
    /// integer in the range \[000,999]
    pub value: Value,
}

impl Line {
    pub const fn new(key: String, value: Value) -> Self {
        Self {
            key,
            value,
        }
    }
    
    pub fn parse(line: &str) -> Result<Self, ParseError> {
        let mut split = line.split(' ').filter(|s| !s.is_empty());
        let key = split.next().ok_or_else(|| ParseError::BadSplit(line.to_owned()))?;
        let value = split.next().ok_or_else(|| ParseError::BadSplit(line.to_owned()))?;

        // assert that there are only two things in the split output
        if split.next().is_some() {
            return Err(ParseError::BadSplit(line.to_owned()));
        }

        let value: Value = Value::parse(value)?;
        let key = key.to_owned();

        Ok(Self {
            key,
            value,
        })
    }

    pub fn serialize(&self) -> String {
        format!("{:63} {}\r\n", self.key, self.value.serialize())
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Value {
    Hide,
    Show,
}

impl Value {
    /// `value` is an integer in the range \[000,999]
    fn parse(value: &str) -> Result<Self, ParseError> {
        match value {
            HIDE_AVATAR_VALUE => Ok(Self::Hide),
            SHOW_AVATAR_VALUE => Ok(Self::Show),
            unknown_value => Err(ParseError::UnknownValue(unknown_value.to_owned())),
        }
    }

    const fn serialize(&self) -> &str {
        match self {
            Self::Hide => HIDE_AVATAR_VALUE,
            Self::Show => SHOW_AVATAR_VALUE,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum ParseError {
    BadSplit(String),
    UnknownValue(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_hide() {
        let actual = Line::parse("usr_6b683acd-31a6-495d-aa46-a73c1349f462                        004").unwrap();
        let expected = Line {
            key: "usr_6b683acd-31a6-495d-aa46-a73c1349f462".to_string(),
            value: Value::Hide,
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_line_show() {
        let actual = Line::parse("usr_6b683acd-31a6-495d-aa46-a73c1349f462                        005").unwrap();
        let expected = Line {
            key: "usr_6b683acd-31a6-495d-aa46-a73c1349f462".to_string(),
            value: Value::Show,
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_line_weird() {
        let actual = Line::parse("2ZaOGztkpc                                                      005").unwrap();
        let expected = Line {
            key: "2ZaOGztkpc".to_string(),
            value: Value::Show,
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_line_unknown_value() {
        let actual = Line::parse("2ZaOGztkpc                                                      009").unwrap_err();
        let expected = ParseError::UnknownValue("009".to_string());
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_line_bad_split_not_enough() {
        let actual = Line::parse("2ZaOGztkpc").unwrap_err();
        let expected = ParseError::BadSplit("2ZaOGztkpc".to_string());
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_line_bad_split_too_many() {
        let actual = Line::parse("2ZaOGztkpc                                                      foo bar").unwrap_err();
        let expected = ParseError::BadSplit("2ZaOGztkpc                                                      foo bar".to_string());
        assert_eq!(actual, expected);
    }
}
