// This file is part of hooligan and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2025 Michael Ripley

use std::process::Command;

fn main() {
    // record git commit hash
    {
        let output = Command::new("git").args(["rev-parse", "HEAD"]).output().unwrap();
        let git_commit_hash = String::from_utf8(output.stdout).unwrap();
        println!("cargo:rustc-env=GIT_COMMIT_HASH={git_commit_hash}");
    }
}
