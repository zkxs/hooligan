// This file is part of hooligan and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2024 Michael Ripley

//! Logging-related utilities

use std::{fmt, fs, io};
use std::fmt::{Display, Formatter};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::SystemTime;
use directories::ProjectDirs;
use file_rotate::{ContentLimit, FileRotate};
use file_rotate::suffix::AppendCount;

type LogWrite = BufWriter<FileRotate<AppendCount>>;

pub struct LogFile {
    write: LogWrite,
}

impl LogFile {
    fn new(write: LogWrite) -> Self {
        Self { write }
    }

    pub fn write_fmt(&mut self, args: fmt::Arguments<'_>) {
        write!(self.write, "{}: ", CurrentTime).expect("failed to write log timestamp");
        self.write.write_fmt(args).expect("failed to write log arguments");
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.write.flush()
    }
}

pub fn get_logger() -> io::Result<LogFile> {
    let file_rotate = FileRotate::new(
        get_log_file_prefix()?,
        AppendCount::new(3),
        ContentLimit::BytesSurpassed(1024 * 1024 * 10),
    );
    Ok(LogFile::new(BufWriter::new(file_rotate)))
}

fn get_log_file_prefix() -> io::Result<PathBuf> {
    let mut log_file_prefix_path = create_log_dir_path()?;
    log_file_prefix_path.push("hooligan.log");
    Ok(log_file_prefix_path)
}

fn create_log_dir_path() -> io::Result<PathBuf> {
    let log_dir_path: PathBuf = get_log_dir()?;
    fs::create_dir_all(log_dir_path.as_path())?;
    Ok(log_dir_path)
}

fn get_log_dir() -> io::Result<PathBuf> {
    Ok(
        ProjectDirs::from("zkxs.dev", "", "hooligan")
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "failed to find valid project directory"))?
            .data_local_dir()
            .join("logs")
    )
}

/// Handles displaying the current time in a minimally expensive way
struct CurrentTime;

impl Display for CurrentTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match SystemTime::UNIX_EPOCH.elapsed() {
            Ok(current_time) => write!(f, "{}", current_time.as_secs()),
            Err(e) => write!(f, "-{}", e.duration().as_secs())
        }
    }
}
