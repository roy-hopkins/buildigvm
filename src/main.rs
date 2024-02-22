// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2023 SUSE LLC
//
// Author: Roy Hopkins <rhopkins@suse.de>

use igvm_builder::IgvmBuilder;
use std::error::Error;

mod cmd_options;
mod igvm_builder;
mod ovmf_firmware;
mod vmsa;

fn main() -> Result<(), Box<dyn Error>> {
    let builder = IgvmBuilder::new()?;
    builder.build()?;
    Ok(())
}
