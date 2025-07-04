// This file is part of hooligan and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2025 Michael Ripley

// don't pop up a weird terminal window
#![cfg_attr(not(test), windows_subsystem = "windows")]

use crate::config::Config;
use crate::local_player_moderations as moderation;
use crate::transaction::{Transaction, Value as TransactionValue};
use bstr::io::BufReadExt;
use directories::ProjectDirs;
use std::ffi::OsString;
use std::fs::{self, DirEntry, File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::num::TryFromIntError;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::{env, io};

mod config;
mod local_player_moderations;
mod logging;
mod transaction;

fn main() -> ExitCode {
    // toss some global-state type things into a struct to make them easier to access
    let project_dirs = get_project_dirs().expect("failed to get project directory");
    let log = logging::get_logger(&project_dirs).expect("failed to open log file for writing");
    Hooligan { log, project_dirs }.run()
}

#[allow(dead_code)] // lint misses usage in debug printing this error
#[derive(Debug)]
enum Error {
    Io(io::Error),
    ShowHideParse(moderation::ParseError),
    TransactionParse(transaction::ParseError),
    EnvironmentVar(env::VarError),
    U64FromInt(TryFromIntError),
    BadFilename(OsString),
    ConfigLoad(config::Error),
}

struct Hooligan {
    log: logging::LogWriter,
    project_dirs: ProjectDirs,
}

impl Hooligan {
    fn run(mut self) -> ExitCode {
        let lockfile = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.project_dirs.data_local_dir().join("hooligan.lock"));
        let exit_code = match lockfile {
            Ok(lockfile) => {
                let mut lock = fd_lock::RwLock::new(lockfile);
                match lock.try_write() {
                    Ok(lock) => {
                        let exit_code = match self.run_checked() {
                            Ok(()) => {
                                writeln!(self.log, "done");
                                ExitCode::SUCCESS
                            }
                            Err(e) => {
                                writeln!(self.log, "{e:?}");
                                ExitCode::FAILURE // 1 on Windows
                            }
                        };

                        drop(lock);

                        exit_code
                    }
                    Err(e) => match e.kind() {
                        io::ErrorKind::WouldBlock => {
                            writeln!(self.log, "aborting because hooligan.lock is owned by another process");
                            ExitCode::from(3)
                        }
                        _ => {
                            writeln!(self.log, "unknown failure acquiring hooligan.lock: {e:?}");
                            ExitCode::from(4)
                        }
                    },
                }
            }
            Err(e) => {
                writeln!(self.log, "error opening hooligan.lock {e:?}");
                ExitCode::from(2)
            }
        };
        self.log.flush().expect("failed to flush log buffer to disk");
        exit_code
    }

    fn run_checked(&mut self) -> Result<(), Error> {
        writeln!(
            self.log,
            "starting {} version {} {}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            env!("GIT_COMMIT_HASH")
        );

        // read config
        let config = self.load_config();

        // iterate over all *.vrcset files
        let read_dir = fs::read_dir(get_local_player_moderations_path()?).map_err(Error::Io)?;
        for dir_entry in read_dir {
            let dir_entry = dir_entry.map_err(Error::Io)?;
            self.process_file(dir_entry, &config)?;
        }

        // launch the VRChat process
        self.spawn_process()?;

        Ok(())
    }

    /// Load config from disk
    fn load_config(&mut self) -> Config {
        let config_dir = self.project_dirs.config_local_dir();
        let config_path = config_dir.join("config.props");
        if config_path.is_file() {
            match Config::load(config_path.as_path()).map_err(Error::ConfigLoad) {
                Ok(config) => config,
                Err(e) => {
                    writeln!(self.log, "failed to load config and falling back to default: {e:?}");
                    Config::default()
                }
            }
        } else {
            let config = Config::default();
            if let Err(e) = fs::create_dir_all(config_dir) {
                writeln!(self.log, "error creating config directory: {e:?}");
            }
            if let Err(e) = config.serialize(config_path.as_path()) {
                writeln!(self.log, "error saving default config: {e:?}");
            }
            config
        }
    }

    /// process a *.vrcset file
    fn process_file(&mut self, dir_entry: DirEntry, config: &Config) -> Result<(), Error> {
        if dir_entry.file_name().as_encoded_bytes().ends_with(b".vrcset") {
            let vrcset_path = dir_entry.path();
            if vrcset_path.is_file() {
                // calculate some paths and filenames
                let mut transaction_log_path = self.project_dirs.data_local_dir().join("history");
                fs::create_dir_all(transaction_log_path.as_path()).map_err(Error::Io)?;
                let vrcset_os_filename = vrcset_path.file_name().unwrap();
                let vrcset_filename = vrcset_os_filename
                    .to_str()
                    .ok_or_else(|| Error::BadFilename(vrcset_os_filename.to_owned()))?;
                let transaction_log_filename = vrcset_filename
                    .split_once('.')
                    .ok_or_else(|| Error::BadFilename(vrcset_os_filename.to_owned()))?
                    .0
                    .to_string()
                    + ".history";
                transaction_log_path.push(transaction_log_filename);

                // read ordered transaction log counting shows since last hide into a map
                let transaction_log_file = OpenOptions::new()
                    .read(true)
                    .append(true)
                    .create(true)
                    .open(transaction_log_path.as_path())
                    .map_err(Error::Io)?;
                let mut shows_since_last_hide = if transaction_log_path.is_file() {
                    Some(transaction::read_log(&transaction_log_file)?)
                } else {
                    None
                };

                // stream changes to vrcset file. By this I mean we are interleaving reads and writes to the same file.
                let mut removed: u32 = 0; // track removed lines
                let mut retained: u32 = 0; // track retained lines that we would have normally removed, if not for the threshold
                let mut pending_transactions: Vec<Transaction> = Vec::new(); // track difference between previous data and current data
                let lines_to_remove = {
                    let vrcset_file = OpenOptions::new()
                        .read(true)
                        .open(vrcset_path.as_path())
                        .map_err(Error::Io)?;
                    let line_reader = BufReader::new(vrcset_file).byte_lines();
                    line_reader.map(|maybe_line| {
                        // parse the lines handling errors
                        match maybe_line {
                            Ok(line) => moderation::Line::parse(&line).map_err(Error::ShowHideParse),
                            Err(e) => Err(Error::Io(e)),
                        }
                    })
                }
                .filter(|line| {
                    line.as_ref().map_or(true, |line| {
                        // retain errors

                        // number of times user was shown since last hide OR None if there is no data
                        let shows = shows_since_last_hide.as_mut().and_then(|map| map.remove(line.key()));

                        match line.value() {
                            moderation::Value::Hide => {
                                // we read a Hide from the vrcset file
                                if shows.map(|shows| !shows.is_hidden()).unwrap_or(true) {
                                    // if user was NOT last known to be hidden, record this manual hide
                                    pending_transactions.push(Transaction::new(
                                        line.key().to_owned().into_boxed_slice(),
                                        TransactionValue::ManualHide,
                                    ));
                                }
                                true // retain hidden user entries
                            }
                            moderation::Value::Show => {
                                // we read a Show from the vrcset file
                                // if we see a manual show in this block we need to consider it in the total show count
                                let extra_shows = if shows.as_ref().map(|shows| !shows.is_shown()).unwrap_or(true) {
                                    // if user was NOT last known to be shown, record this manual show
                                    pending_transactions.push(Transaction::new(
                                        line.key().to_owned().into_boxed_slice(),
                                        TransactionValue::ManualShow,
                                    ));
                                    1
                                } else {
                                    0
                                };

                                // check if we've shown this user enough times that the show should stick
                                if shows
                                    .map(|shows| shows.count() + extra_shows < config.auto_hide_threshold)
                                    .unwrap_or(true)
                                {
                                    // not enough shows; reset the user
                                    pending_transactions
                                        .push(Transaction::new(line.key().into(), TransactionValue::AutoReset));
                                    removed += 1;
                                    false // remove entry
                                } else {
                                    // enough shows; retain the user
                                    retained += 1;
                                    true // retain entry
                                }
                            }
                        }
                    })
                });
                {
                    let vrcset_file = OpenOptions::new()
                        .write(true)
                        .open(vrcset_path.as_path())
                        .map_err(Error::Io)?;
                    self.write_lines(&vrcset_file, lines_to_remove, true)?; // overwrite the vrcset file
                }
                writeln!(
                    self.log,
                    "removed {removed} and retained {retained} shown user entries from {vrcset_filename}"
                );

                // handle any remaining entries in the map
                if let Some(shows_since_last_hide) = shows_since_last_hide {
                    let mut shown: u32 = 0;

                    let (default_lines, non_default_lines): (Vec<_>, Vec<_>) = shows_since_last_hide
                        .into_iter()
                        .partition(|(_, state)| state.is_default());

                    // handle manual non-default -> default transitions
                    non_default_lines.into_iter().for_each(|(key, _)| {
                        pending_transactions.push(Transaction::new(key, TransactionValue::ManualReset))
                    });

                    // handle case where the auto-hide threshold has lowered: we need to go back and re-show previously reset users
                    let mut lines_to_show = default_lines
                        .into_iter()
                        .filter(|(_, show_hide_count)| show_hide_count.count() >= config.auto_hide_threshold)
                        .map(|(key, _)| {
                            shown += 1;
                            pending_transactions.push(Transaction::new(key.clone(), TransactionValue::AutoShow));
                            Ok(moderation::Line::new(key, moderation::Value::Show))
                        })
                        .peekable();
                    if lines_to_show.peek().is_some() {
                        // reopen file in append mode and write these lines
                        let vrcset_file = OpenOptions::new()
                            .append(true)
                            .open(vrcset_path.as_path())
                            .map_err(Error::Io)?;
                        self.write_lines(&vrcset_file, lines_to_show, false)?;
                        writeln!(self.log, "added {shown} shown user entries to {vrcset_filename}");
                    }
                }

                // persist changes to transaction log
                writeln!(self.log, "about to record {} transactions", pending_transactions.len());
                transaction::write_log(&transaction_log_file, pending_transactions)?;
            }
        }

        Ok(())
    }

    /// Write lines to a *.vrcset file. It is NOT required to open the file in truncate mode: truncation is handled by this function.
    ///
    /// Note that `line_iter` is allowed to lazily perform reads to the same file we are writing to, albeit using a distinct file handle.
    ///
    /// **SPOOKY BEHAVIOR WARNING:** If this function fails partway through it will leave the *.vrcset file in a corrupted state.
    /// This can happen in edge cases including:
    /// - out of memory: (various small heap allocations occur in the function)
    /// - out of storage: (bytes are being written to disk)
    /// - weird filesystem edge cases, such as file permissions being change mid-write.
    ///
    /// Probably the easiest safety thing here would just be to have the filesystem make a backup copy before I do any writes,
    /// but that is not presently implement.
    fn write_lines<T: Iterator<Item = Result<moderation::Line, Error>>>(
        &mut self,
        file: &File,
        line_iter: T,
        truncate: bool,
    ) -> Result<(), Error> {
        let mut writer = BufWriter::new(file);
        let mut size: u64 = 0;
        for line in line_iter {
            match line {
                Ok(line) => {
                    size +=
                        u64::try_from(line.serialize(&mut writer).map_err(Error::Io)?).map_err(Error::U64FromInt)?;
                }
                Err(Error::ShowHideParse(e)) => {
                    size += u64::try_from(e.serialize(&mut writer).map_err(Error::Io)?).map_err(Error::U64FromInt)?;
                    writeln!(self.log, "not touching line due to parse error: {e:?}");
                }
                Err(e) => {
                    // We got some kind of IO Error (or an unexpected error type got passed in)
                    // This is awful and has a high chance of file corruption, but the panic might save us of the BufWriter hasn't flushed yet
                    writeln!(
                        self.log,
                        "error {e:?} while streaming file modifications; I will now panic"
                    );
                    panic!("error {e:?} while streaming file modifications");
                }
            }
        }
        writer.flush().map_err(Error::Io)?;
        if truncate {
            file.set_len(size).map_err(Error::Io)?;
        }
        Ok(())
    }

    /// launch the provided process
    fn spawn_process(&mut self) -> Result<(), Error> {
        let mut args = env::args().skip(1); // we skip the first arg because it's just a path to this executable
        if let Some(command) = args.next() {
            // we got args, blindly run them as a command
            let mut command = Command::new(command);
            command.args(args);
            writeln!(self.log, "spawning {command:?}");
            let _ = command.spawn().map_err(Error::Io)?;
        }
        Ok(())
    }
}

/// calculate the path to %UserProfile%\AppData\LocalLow\VRChat\VRChat\LocalPlayerModerations
///
/// **Platform support**: This contains a hardcoded path that only works on Windows... which should be fine as VRChat
/// only supports Windows.
fn get_local_player_moderations_path() -> Result<PathBuf, Error> {
    let user_profile_path = env::var("UserProfile").map_err(Error::EnvironmentVar)?;
    let mut local_player_moderations_path = PathBuf::from(user_profile_path);
    local_player_moderations_path.push("AppData");
    local_player_moderations_path.push("LocalLow");
    local_player_moderations_path.push("VRChat");
    local_player_moderations_path.push("VRChat");
    local_player_moderations_path.push("LocalPlayerModerations");
    Ok(local_player_moderations_path)
}

fn get_project_dirs() -> Result<ProjectDirs, io::Error> {
    let project_dirs = ProjectDirs::from("zkxs.dev", "", "hooligan")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "failed to find valid project directory"))?;
    Ok(project_dirs)
}
