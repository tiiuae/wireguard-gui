use std::io::{self, Result, Write};
use std::process::*;

pub fn generate_private_key() -> Result<String> {
    let output = Command::new("wg")
        .arg("genkey")
        .stdout(Stdio::piped())
        .output()?;

    String::from_utf8(output.stdout)
        .map(|s| s.trim().into())
        .map_err(|_| io::Error::other("Could not convert output of `wg genkey` to utf-8 string."))
}

pub fn generate_public_key(priv_key: String) -> Result<String> {
    let mut child = Command::new("wg")
        .arg("pubkey")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    std::thread::spawn(move || {
        stdin
            .write_all(priv_key.trim().as_bytes())
            .expect("Failed to write to stdin");
    });

    let output = child.wait_with_output().expect("Failed to read stdout");

    String::from_utf8(output.stdout)
        .map(|s| s.trim().into())
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::Other,
                "Could not convert output of `wg pubkey` to utf-8 string.",
            )
        })
}
