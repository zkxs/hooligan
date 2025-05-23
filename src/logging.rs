// This file is part of hooligan and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2025 Michael Ripley

//! Logging-related utilities

use std::{fmt, fs, io};
use std::fmt::{Display, Formatter};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::SystemTime;

use directories::ProjectDirs;

/// maximum number of log files to retain, including the latest
const MAX_LOG_FILES: u32 = 10;

/// `new_index - old_index` threshold above which we delete the oldest file
/// if there are 10 log files on disk, then `new_index - old_index == 9`.
/// Therefore, a threshold of `8` is appropriate to retain 10 files.
const MAX_LOG_INDEX_DIFFERENCE: u32 = MAX_LOG_FILES - 2;

pub struct LogWriter {
    write: BufWriter<File>,
}

impl LogWriter {
    /// evil hack to write timestamps in logs
    pub fn write_fmt(&mut self, args: fmt::Arguments<'_>) {
        write!(self.write, "{}: ", CurrentTime).expect("failed to write log timestamp");
        self.write.write_fmt(args).expect("failed to write log arguments");
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.write.flush()
    }
}

/// Get a logger that can be used with `writeln!()` and logs to a rotating file.
pub fn get_logger(project_dirs: &ProjectDirs) -> io::Result<LogWriter> {
    let mut open_options = OpenOptions::new();
    open_options.append(true);
    open_options.create_new(true);
    let file = open_options.open(get_log_file(project_dirs)?)?;
    let buf_writer = BufWriter::new(file);
    Ok(LogWriter { write: buf_writer })
}

/// Get the log file
fn get_log_file(project_dirs: &ProjectDirs) -> io::Result<PathBuf> {
    let mut log_file_prefix_path = create_log_dir_path(project_dirs)?;
    let dir_iter = fs::read_dir(log_file_prefix_path.as_path())?;

    // scan all files and record information about oldest and newest files
    let mut oldest_log_file: Option<LogFile> = None;
    let mut newest_log_index: Option<u32> = None;
    for entry in dir_iter {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.starts_with("hooligan.") && filename.ends_with(".log") {
                    let split: Vec<&str> = filename.split('.').collect();
                    if split.len() == 3 {
                        let index = split[1];
                        let index: Option<u32> = index.parse().ok();
                        if let Some(index) = index {
                            if let Some(maybe_oldest) = &oldest_log_file {
                                if index < maybe_oldest.index {
                                    oldest_log_file = Some(LogFile{ index, path: entry.path()})
                                }
                            } else {
                                oldest_log_file = Some(LogFile{ index, path: entry.path()})
                            }
                            if let Some(maybe_newest) = &newest_log_index {
                                if index > *maybe_newest {
                                    newest_log_index = Some(index);
                                }
                            } else {
                                newest_log_index = Some(index);
                            }
                        }
                    }
                }
            }
        }
    }

    // handle deleting the oldest log file if we are at the max log file limit
    if let (Some(oldest_log_file), Some(newest_log_index)) = (oldest_log_file, newest_log_index) {
        if newest_log_index - oldest_log_file.index > MAX_LOG_INDEX_DIFFERENCE {
            fs::remove_file(oldest_log_file.path)?;
        }
    }

    // create the new log file
    let index = newest_log_index.map(|index| index + 1).unwrap_or(0);
    log_file_prefix_path.push(format!("hooligan.{}.log", index));
    Ok(log_file_prefix_path)
}

/// Ensure logging directory exists, and return it
fn create_log_dir_path(project_dirs: &ProjectDirs) -> io::Result<PathBuf> {
    let log_dir_path: PathBuf = get_log_dir(project_dirs);
    fs::create_dir_all(log_dir_path.as_path())?;
    Ok(log_dir_path)
}

/// Get path of logging directory
fn get_log_dir(project_dirs: &ProjectDirs) -> PathBuf {
    project_dirs.data_local_dir().join("logs")
}

/// Handles displaying the current time in a minimally expensive way
struct CurrentTime;

impl Display for CurrentTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match SystemTime::UNIX_EPOCH.elapsed() {
            Ok(current_time) => write!(f, "{}", current_time.as_secs()),
            Err(e) => write!(f, "-{}", e.duration().as_secs())
        }
    }
}

struct LogFile {
    index: u32,
    path: PathBuf,
}
