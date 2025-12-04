/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use clap::Parser;
use clap::ValueEnum;
use lazy_static::lazy_static;
use std::path::PathBuf;
use std::str;
lazy_static! {
    static ref CLI_ARGS: Args = {
        let args = Args::parse();
        println!("{args:?}");
        args
    };
}

#[derive(ValueEnum, Default, Debug, Clone, Copy, PartialEq)]
pub enum LogOutput {
    #[default]
    Syslog,
    Stdout,
}

/// Wireguard GUI for Ghaf
#[derive(Parser, Debug)]
#[command(name = "Wireguard GUI")]
#[command(about = "Wireguard Graphical User Interface")]
#[command(long_about = None)]
struct Args {
    /// Log severity
    #[arg(long, default_value_t = log::Level::Info)]
    pub log_level: log::Level,

    /// Log output
    #[arg(long, value_enum, default_value_t)]
    pub log_output: LogOutput,

    /// Path to the Wireguard files
    #[arg(long, default_value = "/etc/wireguard")]
    app_dir: PathBuf,

    /// Owner of the config files
    #[arg(long, default_value = "root")]
    pub config_owner: String,

    /// Owner group of the config files
    #[arg(long, default_value = "root")]
    pub config_owner_group: String,
}

pub fn get_log_level_output() -> log::Level {
    CLI_ARGS.log_level
}

pub fn get_log_output() -> LogOutput {
    CLI_ARGS.log_output
}

pub fn get_configs_dir() -> PathBuf {
    CLI_ARGS.app_dir.join("configs")
}

pub fn get_scripts_dir() -> PathBuf {
    CLI_ARGS.app_dir.join("scripts")
}

pub fn get_config_file_owner() -> &'static str {
    &CLI_ARGS.config_owner
}

pub fn get_config_file_owner_group() -> &'static str {
    &CLI_ARGS.config_owner_group
}
