use std::fs;
use std::io::{self, Error, Result, Write};
use std::process::*;

use crate::config::{parse_config, WireguardConfig};

pub const TUNNELS_PATH: &str = "/etc/wireguard";

pub fn load_existing_configurations() -> Result<Vec<WireguardConfig>> {
    let mut cfgs = vec![];

    for entry in fs::read_dir(TUNNELS_PATH)? {
        let file = entry?;
        if file.file_type()?.is_file() {
            let file_path = file.path();
            let file_content = fs::read_to_string(&file_path)?;
            let mut cfg = parse_config(&file_content).map_err(Error::other)?;
            if cfg.interface.name.is_none() {
                if let Some(file_name) = file_path.file_stem().and_then(|n| n.to_str()) {
                    cfg.interface.name = Some(file_name.to_string());
                }
            }
            cfgs.push(cfg);
        }
    }

    Ok(cfgs)
}

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

pub fn generate_preshared_key() -> Result<String> {
    let output = Command::new("wg")
        .arg("genpsk")
        .stdout(Stdio::piped())
        .output()?;

    String::from_utf8(output.stdout)
        .map(|s| s.trim().into())
        .map_err(|_| io::Error::other("Could not convert output of `wg genpsk` to utf-8 string."))
}
