// This file is part of hooligan and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2024 Michael Ripley

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

use ParseError::UnknownValue;

use crate::Error;

const AUTO_RESET: &str = "AUTO_RESET";
const AUTO_SHOW: &str = "AUTO_SHOW";
const MANUAL_HIDE: &str = "MANUAL_HIDE";
const MANUAL_RESET: &str = "MANUAL_RESET";
const MANUAL_SHOW: &str = "MANUAL_SHOW";

pub struct Transaction {
    pub key: String,
    pub value: Value,
}

impl Transaction {
    pub const fn new(key: String, value: Value) -> Self {
        Self {
            key,
            value,
        }
    }

    pub fn parse(value: &str) -> Result<Self, ParseError> {
        let (key, value) = value.split_once(' ').ok_or_else(|| ParseError::BadSplit(value.to_owned()))?;
        let key = key.to_owned();
        let value = Value::parse(value)?;
        Ok(Self {
            key,
            value,
        })
    }

    pub fn serialize(&self) -> String {
        format!("{} {}\n", self.key, self.value.serialize())
    }
}

pub enum Value {
    AutoReset,
    AutoShow,
    ManualHide,
    ManualReset,
    ManualShow,
}

impl Value {
    fn parse(value: &str) -> Result<Self, ParseError> {
        match value {
            AUTO_RESET => Ok(Self::AutoReset),
            AUTO_SHOW => Ok(Self::AutoShow),
            MANUAL_HIDE => Ok(Self::ManualHide),
            MANUAL_RESET => Ok(Self::ManualReset),
            MANUAL_SHOW => Ok(Self::ManualShow),
            unknown => Err(UnknownValue(unknown.to_owned())),
        }
    }

    const fn serialize(&self) -> &str {
        match self {
            Self::AutoReset => AUTO_RESET,
            Self::AutoShow => AUTO_SHOW,
            Self::ManualHide => MANUAL_HIDE,
            Self::ManualReset => MANUAL_RESET,
            Self::ManualShow => MANUAL_SHOW,
        }
    }
}

#[allow(dead_code)] // lint misses usage in debug printing this error
#[derive(Debug)]
pub enum ParseError {
    BadSplit(String),
    UnknownValue(String),
}

pub struct ShowHideCount {
    count: u32,
    state: ShowHideState,
}

enum ShowHideState {
    Shown,
    Hidden,
    Default,
}

impl ShowHideCount {
    const fn new(count: u32, state: ShowHideState) -> Self {
        Self {
            count,
            state,
        }
    }

    fn reset(&mut self, state: ShowHideState) {
        self.count = 0;
        self.state = state;
    }

    fn increment(&mut self, state: ShowHideState) {
        self.count += 1;
        self.state = state;
    }

    fn set_state(&mut self, state: ShowHideState) {
        self.state = state;
    }

    pub const fn count(&self) -> u32 {
        self.count
    }

    pub const fn is_shown(&self) -> bool {
        matches!(self.state, ShowHideState::Shown)
    }

    pub const fn is_hidden(&self) -> bool {
        matches!(self.state, ShowHideState::Hidden)
    }

    pub const fn is_default(&self) -> bool {
        matches!(self.state, ShowHideState::Default)
    }
}

/// Count shows since last manual hide
pub fn read_log(file: &File) -> Result<HashMap<String, ShowHideCount>, Error> {
    let line_reader = BufReader::new(file).lines();
    let mut map: HashMap<String, ShowHideCount> = HashMap::new();
    for line in line_reader {
        let line = line.map_err(Error::Io)?;
        let transaction = Transaction::parse(&line).map_err(Error::TransactionParse)?;
        match transaction.value {
            Value::AutoReset => {
                // existing show count should be left alone; OTHERWISE absent show count should be initialized to 0
                map.entry(transaction.key)
                    .and_modify(|value| value.set_state(ShowHideState::Default))
                    .or_insert(ShowHideCount::new(0, ShowHideState::Default));
            }
            Value::AutoShow => {
                // existing show count should be left alone; OTHERWISE absent show count should be initialized to 0
                map.entry(transaction.key)
                    .and_modify(|value| value.set_state(ShowHideState::Shown))
                    .or_insert(ShowHideCount::new(0, ShowHideState::Shown));
            }
            Value::ManualHide => {
                // existing show count should be reset; OTHERWISE absent show count should be initialized to 0
                map.entry(transaction.key)
                    .and_modify(|value| value.reset(ShowHideState::Hidden))
                    .or_insert(ShowHideCount::new(0, ShowHideState::Hidden));
            }

            Value::ManualReset => {
                // existing show count should be reset; OTHERWISE absent show count should be initialized to 0
                map.entry(transaction.key)
                    .and_modify(|value| value.reset(ShowHideState::Default))
                    .or_insert(ShowHideCount::new(0, ShowHideState::Default));
            }
            Value::ManualShow => {
                // existing show count should be incremented; OTHERWISE absent show count should be initialized to 1
                map.entry(transaction.key)
                    .and_modify(|value| value.increment(ShowHideState::Shown))
                    .or_insert(ShowHideCount::new(1, ShowHideState::Shown));
            }
        }
    }
    Ok(map)
}

pub fn write_log(file: &File, transaction_log: Vec<Transaction>) -> Result<(), Error> {
    let mut writer = BufWriter::new(file);
    for transaction in transaction_log {
        write!(writer, "{}", transaction.serialize()).map_err(Error::Io)?;
    }
    writer.flush().map_err(Error::Io)?;
    Ok(())
}
