// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2024 SUSE LLC
//
// Author: Roy Hopkins <roy.hopkins@suse.com>

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
pub struct CmdOptions {
    /// Firmware file, e.g. OVMF.fd
    #[arg(short, long)]
    pub firmware: String,

    /// Output filename for the generated IGVM file
    #[arg(short, long)]
    pub output: String,

    #[arg(short, long)]
    pub cpucount: u16,

    /// Print verbose output
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    #[arg(value_enum)]
    pub platform: Platform,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum Platform {
    /// Build an IGVM file compatible with SEV
    Sev,

    /// Build an IGVM file compatible with SEV-ES
    SevEs,
}
