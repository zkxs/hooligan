// This file is part of hooligan and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2024 Michael Ripley

#![windows_subsystem = "windows"] // don't pop up a weird terminal window

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::local_player_moderations as lpmod;

mod local_player_moderations;
mod logging;

fn main() {
    let log = logging::get_logger().expect("failed to open log file for writing");
    Hooligan { log }.run();
}

#[allow(dead_code)] // lint misses usage in debug printing this error
#[derive(Debug)]
enum Error {
    Io(std::io::Error),
    Parse(lpmod::ParseError),
    EnvironmentVar(env::VarError),
}

struct Hooligan {
    log: logging::LogFile,
}

impl Hooligan {
    fn run(mut self) {
        match self.run_checked() {
            Ok(()) => writeln!(self.log, "done"),
            Err(e) => writeln!(self.log, "{e:?}"),
        }
        self.log.flush().expect("failed to flush log buffer to disk");
    }

    fn run_checked(&mut self) -> Result<(), Error> {
        writeln!(self.log, "starting {} version {} {}",
                 env!("CARGO_PKG_NAME"),
                 env!("CARGO_PKG_VERSION"),
                 env!("GIT_COMMIT_HASH"));

        // iterate over all *.vrcset files
        let read_dir = fs::read_dir(get_local_player_moderations_path()?).map_err(Error::Io)?;
        for dir_entry in read_dir {
            let dir_entry = dir_entry.map_err(Error::Io)?;
            if dir_entry.file_name().as_encoded_bytes().ends_with(b".vrcset") {
                let path = dir_entry.path();
                if path.is_file() {
                    let mut removed: u32 = 0;
                    let lines = read_lines(path.as_path())?.into_iter()
                        .filter(|line| {
                            // remove shown avatar entries
                            let remove = matches!(line.value, lpmod::Value::Show);
                            if remove {
                                removed += 1;
                            }
                            !remove
                        });
                    write_lines(path, lines)?;
                    writeln!(self.log, "removed {} shown avatar entries", removed);
                }
            }
        }

        // launch the VRChat process
        let mut args = env::args().skip(1); // we skip the first arg because it's just a path to this executable
        if let Some(command) = args.next() {
            // we got args, blindly run them as a command
            let mut command = Command::new(command);
            command.args(args);
            let _ = command.spawn().map_err(Error::Io)?;
        }

        Ok(())
    }
}

/// calculate the path to %UserProfile%\AppData\LocalLow\VRChat\VRChat\LocalPlayerModerations
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

fn read_lines<P>(path: P) -> Result<Vec<lpmod::Line>, Error>
where
    P: AsRef<Path>,
{
    let file = File::open(path).map_err(Error::Io)?;
    let lines = BufReader::new(file).lines();
    let mut line_vec = Vec::new();
    for line in lines {
        let line = line.map_err(Error::Io)?;
        let line = lpmod::Line::parse(&line).map_err(Error::Parse)?;
        line_vec.push(line);
    }
    Ok(line_vec)
}

fn write_lines<P, T>(path: P, line_iter: T) -> Result<(), Error>
where
    P: AsRef<Path>,
    T: Iterator<Item=lpmod::Line>,
{
    let mut open_options = OpenOptions::new();
    open_options.write(true);
    open_options.truncate(true);

    let file = open_options.open(path).map_err(Error::Io)?;
    let mut writer = BufWriter::new(file);
    for line in line_iter {
        let serialized = line.serialize();
        writer.write(serialized.as_bytes()).map_err(Error::Io)?;
    }
    writer.flush().map_err(Error::Io)?;
    Ok(())
}
