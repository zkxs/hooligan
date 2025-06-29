// This file is part of hooligan and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2025 Michael Ripley

use ParseError::UnknownValue;
use bstr::ByteSlice;
use bstr::io::BufReadExt;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter, Write};

use crate::Error;

const AUTO_RESET: &[u8] = b"AUTO_RESET";
const AUTO_SHOW: &[u8] = b"AUTO_SHOW";
const MANUAL_HIDE: &[u8] = b"MANUAL_HIDE";
const MANUAL_RESET: &[u8] = b"MANUAL_RESET";
const MANUAL_SHOW: &[u8] = b"MANUAL_SHOW";

/// A single transaction from the ordered transaction log hooligan uses to track changes over time
/// to a *.vrcset file
pub struct Transaction {
    /// referenced user
    key: Box<[u8]>,
    /// the event
    value: Value,
}

impl Transaction {
    pub const fn new(key: Box<[u8]>, value: Value) -> Self {
        Self { key, value }
    }

    pub fn parse(value: &[u8]) -> Result<Self, ParseError> {
        let (key, value) = value
            .split_once_str(b" ")
            .ok_or_else(|| ParseError::BadSplit(value.to_owned().into_boxed_slice()))?;
        let key = key.to_owned().into_boxed_slice();
        let value = Value::parse(value)?;
        Ok(Self { key, value })
    }

    pub fn serialize(&self, writer: &mut impl Write) -> io::Result<usize> {
        let mut written = 0;
        written += writer.write(&self.key)?;
        written += writer.write(b" ")?;
        written += writer.write(self.value.serialize())?;
        written += writer.write(b"\n")?;
        Ok(written)
    }
}

/// The value of a transaction. This is the event being logged.
pub enum Value {
    /// hooligan automatically reset a user by deleting the LocalPlayerModeration entry
    AutoReset,
    /// hooligan automatically showed a user by creating a new LocalPlayerModeration entry.
    /// This can happen in certain edge cases: for example, when the configured auto-hide threshold
    /// has lowered since the last time this program ran.
    AutoShow,
    /// a manual user hide was detected by diffing the *.vrcset file
    ManualHide,
    /// a manual user reset was detected by diffing the *.vrcset file
    ManualReset,
    /// a manual user show was detected by diffing the *.vrcset file
    ManualShow,
}

impl Value {
    fn parse(value: &[u8]) -> Result<Self, ParseError> {
        match value {
            AUTO_RESET => Ok(Self::AutoReset),
            AUTO_SHOW => Ok(Self::AutoShow),
            MANUAL_HIDE => Ok(Self::ManualHide),
            MANUAL_RESET => Ok(Self::ManualReset),
            MANUAL_SHOW => Ok(Self::ManualShow),
            unknown => Err(UnknownValue(unknown.to_owned().into_boxed_slice())),
        }
    }

    const fn serialize(&self) -> &[u8] {
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
    BadSplit(Box<[u8]>),
    UnknownValue(Box<[u8]>),
}

/// Current state for a user key, calculated by scanning over the transaction log linearly
pub struct ShowHideCount {
    /// running count of shows since last hide
    count: u32,
    /// current state for this user key
    state: ShowHideState,
}

/// Possible states
enum ShowHideState {
    /// Equivalent to a SHOW *.vrcset entry
    Shown,
    /// Equivalent to a HIDE *.vrcset entry
    Hidden,
    /// Equivalent to no *.vrcset entry
    Default,
}

impl ShowHideCount {
    const fn new(count: u32, state: ShowHideState) -> Self {
        Self { count, state }
    }

    /// Reset the running shows-since-last-hide count and set the state
    const fn reset(&mut self, state: ShowHideState) {
        self.count = 0;
        self.state = state;
    }

    /// Increment the running shows-since-last-hide count and set the state
    const fn increment(&mut self, state: ShowHideState) {
        self.count += 1;
        self.state = state;
    }

    const fn set_state(&mut self, state: ShowHideState) {
        self.state = state;
    }

    /// Number of shows since last hide
    pub const fn count(&self) -> u32 {
        self.count
    }

    /// Is this user key shown?
    pub const fn is_shown(&self) -> bool {
        matches!(self.state, ShowHideState::Shown)
    }

    /// Is this user key hidden?
    pub const fn is_hidden(&self) -> bool {
        matches!(self.state, ShowHideState::Hidden)
    }

    /// Is this user key at the default (aka unset) show/hide state?
    pub const fn is_default(&self) -> bool {
        matches!(self.state, ShowHideState::Default)
    }
}

/// Count shows since last manual hide
pub fn read_log(file: &File) -> Result<HashMap<Box<[u8]>, ShowHideCount>, Error> {
    let line_reader = BufReader::new(file).byte_lines();
    let mut map: HashMap<Box<[u8]>, ShowHideCount> = HashMap::new();
    for line in line_reader {
        let line = line.map_err(Error::Io)?;
        let transaction = Transaction::parse(&line).map_err(Error::TransactionParse)?;
        let map_entry = map.entry(transaction.key);

        // handle the latest read transaction by updating running count and state values for that user key
        match transaction.value {
            Value::AutoReset => {
                // existing show count should be left alone; OTHERWISE absent show count should be initialized to 0
                map_entry
                    .and_modify(|value| value.set_state(ShowHideState::Default))
                    .or_insert(ShowHideCount::new(0, ShowHideState::Default));
            }
            Value::AutoShow => {
                // existing show count should be left alone; OTHERWISE absent show count should be initialized to 0
                map_entry
                    .and_modify(|value| value.set_state(ShowHideState::Shown))
                    .or_insert(ShowHideCount::new(0, ShowHideState::Shown));
            }
            Value::ManualHide => {
                // existing show count should be reset; OTHERWISE absent show count should be initialized to 0
                map_entry
                    .and_modify(|value| value.reset(ShowHideState::Hidden))
                    .or_insert(ShowHideCount::new(0, ShowHideState::Hidden));
            }

            Value::ManualReset => {
                // existing show count should be reset; OTHERWISE absent show count should be initialized to 0
                map_entry
                    .and_modify(|value| value.reset(ShowHideState::Default))
                    .or_insert(ShowHideCount::new(0, ShowHideState::Default));
            }
            Value::ManualShow => {
                // existing show count should be incremented; OTHERWISE absent show count should be initialized to 1
                map_entry
                    .and_modify(|value| value.increment(ShowHideState::Shown))
                    .or_insert(ShowHideCount::new(1, ShowHideState::Shown));
            }
        }
    }
    Ok(map)
}

/// Append transactions to the transaction log file. Note that by "append" I mean it is acceptable
/// to call this on a transaction file opened in append mode which already contains data.
pub fn write_log(file: &File, transaction_log: Vec<Transaction>) -> Result<(), Error> {
    let mut writer = BufWriter::new(file);
    for transaction in transaction_log {
        transaction.serialize(&mut writer).map_err(Error::Io)?;
    }
    writer.flush().map_err(Error::Io)?;
    Ok(())
}
