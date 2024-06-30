// This file is part of hooligan and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2024 Michael Ripley

use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

const AUTO_HIDE_THRESHOLD: &str = "auto_hide_threshold";

pub struct Config {
    /// a user that has been manually shown this many times in a row is exempt from auto hide
    pub auto_hide_threshold: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_hide_threshold: 3,
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let file = File::open(path).map_err(Error::Io)?;
        let reader = BufReader::new(file);
        let mut config = Self::new();
        for line in reader.lines() {
            let line = line.map_err(Error::Io)?;
            config.parse_line(&line)?;
        }
        Ok(config)
    }

    const fn new() -> Self {
        Self {
            auto_hide_threshold: 0,
        }
    }

    fn parse_line(&mut self, line: &str) -> Result<(), Error> {
        let (key, value) = line.split_once('=').ok_or(Error::Split)?;
        match key {
            AUTO_HIDE_THRESHOLD => self.parse_auto_hide_threshold(value),
            _ => Err(Error::Key),
        }
    }

    fn parse_auto_hide_threshold(&mut self, value: &str) -> Result<(), Error> {
        self.auto_hide_threshold = value.parse().map_err(|_| Error::Int)?;
        Ok(())
    }

    pub fn serialize<P: AsRef<Path>>(&self, path: P) -> Result<(), io::Error> {
        let file = File::create_new(path).unwrap();
        let mut writer = BufWriter::new(file);
        writeln!(writer, "{}={}", AUTO_HIDE_THRESHOLD, self.auto_hide_threshold)?;
        writer.flush()
    }
}

#[allow(dead_code)] // lint misses usage in debug printing this error
#[derive(Debug)]
pub enum Error {
    Split,
    Int,
    Key,
    Io(io::Error),
}
