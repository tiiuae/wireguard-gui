/*
    Copyright 2025 TII (SSRC) and the contributors
    SPDX-License-Identifier: Apache-2.0
*/
use clap::Parser;
use clap::ValueEnum;
use lazy_static::lazy_static;
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
}

pub fn get_log_level() -> &'static log::Level {
    &CLI_ARGS.log_level
}

pub fn get_log_output() -> &'static LogOutput {
    &CLI_ARGS.log_output
}
