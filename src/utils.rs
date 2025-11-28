use crate::cli;
/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use crate::config::{WireguardConfig, parse_config};
use log::*;
use std::fs;
use std::io::{self, Read, Result, Write};
use std::path::Path;
use std::process::*;
use std::time::Duration;
use wait_timeout::ChildExt;

pub fn load_existing_configurations() -> Result<(Vec<WireguardConfig>, Option<String>)> {
    let mut cfgs = Vec::new();
    let mut errs = Vec::new();

    for entry in fs::read_dir(cli::get_configs_dir())? {
        let file = entry?;
        if file.file_type()?.is_file() && file.path().extension().is_some_and(|e| e == "conf") {
            let file_path = file.path();
            let Ok(file_content) = fs::read_to_string(&file_path) else {
                let msg = format!("Could not read file: {}", file_path.display());
                error!("{}", msg);
                errs.push(msg);

                continue;
            };

            let Ok(mut cfg) = parse_config(&file_content) else {
                let msg = format!("Could not parse file: {}", file_path.display());
                error!("{}", msg);
                errs.push(msg);
                continue;
            };
            if cfg.interface.name.is_none() 
                && let Some(file_name) = file_path.file_stem().and_then(|n| n.to_str()) {
                    cfg.interface.name = Some(file_name.to_string());
                }
            

            cfgs.push(cfg);
        }
    }

    let combined_errors = if errs.is_empty() {
        None
    } else {
        Some(errs.join("\n"))
    };

    Ok((cfgs, combined_errors))
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

    // Read stdout and stderr
    let mut stdout_buf = String::new();
    if let Some(stdout) = cmd.stdout.as_mut() {
        stdout.read_to_string(&mut stdout_buf)?;
    }

    let mut stderr_buf = String::new();
    if let Some(stderr) = cmd.stderr.as_mut() {
        stderr.read_to_string(&mut stderr_buf)?;
    }

    let combined_output = format!("{}\n{}", stdout_buf, stderr_buf);

    if let Some((cmd_str, status)) = cmd_str.zip(status_code) {
        if status != 0 {
            error!(
                "Cmd: {} failed with status code {}. Output:\n{}",
                cmd_str, status, combined_output
            );
        } else {
            debug!("Cmd: {} succeeded. Output:\n{}", cmd_str, combined_output);
        }
    }

    Ok((status_code, combined_output))
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

/// Returns true if `path` is safe to export:
/// - Absolute path
/// - Has a filename
/// - Parent exists and is a directory
/// - Parent is inside /home
/// - Target file is not a symlink
pub fn validate_export_path(path: &Path) -> bool {
    // Must be absolute
    if !path.is_absolute() {
        return false;
    }

    // Must have a filename
    let _filename = match path.file_name() {
        Some(f) => f,
        None => return false,
    };

    // Must have a parent
    let parent = match path.parent() {
        Some(p) => p,
        None => return false,
    };

    // Canonicalize parent to defeat symlinks
    let parent_real = match parent.canonicalize() {
        Ok(p) if p.is_dir() => p,
        _ => return false,
    };

    // Must be inside /home
    let home_real = match Path::new("/home").canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    if parent_real.strip_prefix(&home_real).is_err() {
        return false;
    }

    // If file exists, it must not be a symlink
    if path.exists() {
        if let Ok(metadata) = fs::symlink_metadata(path) {
            if metadata.file_type().is_symlink() {
                return false;
            }
        } else {
            return false; // cannot read metadata
        }
    }

    true
}
