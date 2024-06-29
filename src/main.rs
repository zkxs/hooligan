use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, stdin, Write};

use crate::local_player_moderations as lpmod;

mod local_player_moderations;

fn main() -> Result<(), Error> {
    let path = r"C:\Users\runtime\AppData\LocalLow\VRChat\VRChat\LocalPlayerModerations\usr_6947a91b-1f7d-4f22-bb70-727a4837de52-show-hide-user.vrcset";
    let lines = read_lines(path)?.into_iter()
        .filter(|line| matches!(line.value, lpmod::Value::Hide));
    write_lines(path, lines)?;

    let args: Vec<String> = std::env::args().collect();
    for arg in args {
        println!("{arg}");
    }

    // Read a single byte and discard
    let _ = stdin().read(&mut [0u8]).unwrap();

    Ok(())
}

fn read_lines(path: &str) -> Result<Vec<lpmod::Line>, Error> {
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

fn write_lines<T>(path: &str, line_iter: T) -> Result<(), Error>
    where T: Iterator<Item=lpmod::Line> {
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

#[derive(Debug)]
enum Error {
    Io(std::io::Error),
    Parse(lpmod::ParseError),
}
