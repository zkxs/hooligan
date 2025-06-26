// This file is part of hooligan and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2025 Michael Ripley

use rand::Rng;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};

/// This is a sanity check to make sure that interleaving reads and writes to the same file works as expected.
///
/// Turns out it's fine, so long as you don't try and abuse the same file handle for both, as the file offset is stored
/// per-handle.
#[test]
fn interleaved_read_write() {
    let random_id = rand::rng()
        .sample_iter(rand::distr::Alphanumeric)
        .take(12)
        .map(|value| value as char)
        .collect::<String>();
    let mut filename = "hooligan_test_".to_string();
    filename.push_str(&random_id);
    let mut temp_file_path = std::env::temp_dir();
    temp_file_path.push(filename);

    // initial data write
    {
        let mut initial_write_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_file_path)
            .expect("failed to create temp file");
        initial_write_file
            .write_all(b"foo\nbar\nbaz\n")
            .expect("failed to write temp file");
        initial_write_file.flush().expect("failed to flush temp file");
    }

    // weird streaming stuff
    {
        let read_file = OpenOptions::new()
            .read(true)
            .open(&temp_file_path)
            .expect("failed to open temp file");
        let write_file = OpenOptions::new()
            .write(true)
            .open(&temp_file_path)
            .expect("failed to open temp file");
        // use artificially small capacities here to test edge cases were we have to do multiple interleaved read/write calls
        let lines = BufReader::with_capacity(2, &read_file).lines();
        let mut writer = BufWriter::with_capacity(1, &write_file);
        let mut size: u64 = 0;
        for line in lines {
            let line = line.expect("failed to read line");
            if &line != "bar" {
                size += writer.write(line.as_bytes()).expect("failed to write line") as u64;
            }
        }
        writer.flush().expect("failed to flush writer");
        write_file.set_len(size).expect("failed to truncate temp file");
    }

    let mut data = Vec::new();
    {
        let mut read_file = OpenOptions::new()
            .read(true)
            .open(&temp_file_path)
            .expect("failed to open temp file");

        read_file.read_to_end(&mut data).expect("failed to read temp file");
    }

    // note that if the test fails before this line is reached then the temp file will be left around until the OS deals with it
    std::fs::remove_file(&temp_file_path).expect("failed to delete temp file");

    assert_eq!(data, b"foobaz");
}
