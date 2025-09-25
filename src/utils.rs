use crate::cli;
/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use crate::config::{parse_config, WireguardConfig};
use log::*;
use std::fs;
use std::io::{self, Read, Result, Write};
use std::process::*;
use std::time::Duration;
use wait_timeout::ChildExt;

pub fn load_existing_configurations() -> Result<Vec<WireguardConfig>> {
    let mut cfgs = vec![];

    for entry in fs::read_dir(cli::get_configs_dir())? {
        let file = entry?;
        if file.file_type()?.is_file() && file.path().extension().is_some_and(|e| e == "conf") {
            let file_path = file.path();
            let Ok(file_content) = fs::read_to_string(&file_path) else {
                error!("Could not read file: {}", file_path.display());
                continue;
            };

            let Ok(mut cfg) = parse_config(&file_content) else {
                error!("Could not parse file: {}", file_path.display());
                continue;
            };
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

    if output.stdout.is_empty() {
        return Err(io::Error::other("Failed to generate public key"));
    }

    String::from_utf8(output.stdout)
        .map(|s| s.trim().into())
        .map_err(|_| io::Error::other("Could not convert output of `wg pubkey` to utf-8 string."))
}

/// Run a command with a timeout and return the exit status and stdout output.
pub fn wait_cmd_with_timeout(
    mut cmd: Child,
    timeout: u64,
    cmd_str: Option<&str>,
) -> io::Result<(Option<i32>, String)> {
    let timeout = Duration::from_secs(timeout);

    // Wait for the process to exit or timeout
    let status_code = match cmd.wait_timeout(timeout)? {
        Some(status) => {
            cmd.kill()?;
            status.code()
        }
        None => {
            // Process hasn't exited yet, kill it and get the status code
            debug!("Killing the process: {:?}", cmd);
            cmd.kill()?;
            cmd.wait()?.code()
        }
    };

    // Read stdout after killing the process
    let mut cmd_response = String::new();

    // Borrow stdout and read the output into `cmd_response`
    if let Some(stdout) = cmd.stdout.as_mut() {
        stdout.read_to_string(&mut cmd_response)?;
    }

    if let (Some(cmd_str), Some(status_code)) = (cmd_str, status_code) {
        if status_code != 0 {
            error!(
                "Cmd: {} failed with status code: {status_code},response:{cmd_response}",
                cmd_str
            );
        } else {
            debug!("Cmd: {} is successful ,response:{cmd_response}", cmd_str);
        }
    }

    // Return both the status code and the output
    Ok((status_code, cmd_response))
}

pub fn is_ip_valid(ip: Option<&str>) -> bool {
    if let Some(ip_str) = ip {
        let trimmed = ip_str.trim();
        if !trimmed.is_empty() {
            return trimmed.parse::<ipnetwork::IpNetwork>().is_ok();
        }
    }

    false
}
