// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2024 SUSE LLC
//
// Author: Roy Hopkins <roy.hopkins@suse.com>

use std::error::Error;
use std::fs::File;
use std::io::Write;

use clap::Parser;
use igvm::{
    IgvmDirectiveHeader, IgvmFile, IgvmInitializationHeader, IgvmPlatformHeader, IgvmRevision,
};
use igvm_defs::{IgvmPlatformType, IGVM_VHS_SUPPORTED_PLATFORM};

use crate::cmd_options::{self, CmdOptions};
use crate::ovmf_firmware::OvmfFirmware;
use crate::vmsa::{construct_ap_vmsa, construct_bsp_vmsa};

const COMPATIBILITY_MASK: u32 = 1;

pub struct IgvmBuilder {
    options: CmdOptions,
    firmware: OvmfFirmware,
    platforms: Vec<IgvmPlatformHeader>,
    initialization: Vec<IgvmInitializationHeader>,
    directives: Vec<IgvmDirectiveHeader>,
}

impl IgvmBuilder {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let options = CmdOptions::parse();
        let firmware =
            OvmfFirmware::parse(&options.firmware, COMPATIBILITY_MASK, options.platform)?;
        Ok(Self {
            options,
            firmware,
            platforms: vec![],
            initialization: vec![],
            directives: vec![],
        })
    }

    pub fn build(mut self) -> Result<(), Box<dyn Error>> {
        self.build_initialization()?;
        self.build_directives()?;
        self.build_platforms();

        // Separate the directive pages out from the others so we can populate them last.
        let (mut pages, others): (Vec<_>, Vec<_>) = self
            .directives
            .iter()
            .cloned()
            .partition(Self::filter_pages);

        self.directives = others;
        self.directives.append(&mut pages);

        if self.options.verbose {
            let fw_info = self.firmware.get_fw_info();
            println!("{fw_info:#X?}");
        }

        let file = IgvmFile::new(
            IgvmRevision::V1,
            self.platforms,
            self.initialization,
            self.directives,
        )
        .map_err(|e| {
            eprintln!("Failed to create output file");
            e
        })?;

        let mut binary_file = Vec::new();
        file.serialize(&mut binary_file)?;

        let mut output = File::create(&self.options.output).map_err(|e| {
            eprintln!("Failed to create output file {}", self.options.output);
            e
        })?;
        output.write_all(binary_file.as_slice()).map_err(|e| {
            eprintln!("Failed to write output file {}", self.options.output);
            e
        })?;
        Ok(())
    }

    fn build_platforms(&mut self) {
        let platform_type = match self.options.platform {
            cmd_options::Platform::Sev => IgvmPlatformType::SEV,
            cmd_options::Platform::SevEs => IgvmPlatformType::SEV_ES,
            cmd_options::Platform::SevSnp => IgvmPlatformType::SEV_SNP,
            cmd_options::Platform::Native => IgvmPlatformType::NATIVE,
        };
        self.platforms.push(IgvmPlatformHeader::SupportedPlatform(
            IGVM_VHS_SUPPORTED_PLATFORM {
                compatibility_mask: COMPATIBILITY_MASK,
                highest_vtl: 0,
                platform_type,
                platform_version: 1,
                shared_gpa_boundary: 0,
            },
        ));
    }

    fn build_directives(&mut self) -> Result<(), Box<dyn Error>> {
        // Populate firmware directives.
        self.directives
            .extend_from_slice(self.firmware.directives());

        match self.options.platform {
            cmd_options::Platform::SevEs | cmd_options::Platform::SevSnp => {
                // Build VMSAs for the required number of processors
                self.directives.push(construct_bsp_vmsa(
                    0xFFFFFFFFF000,
                    COMPATIBILITY_MASK,
                    self.options.platform,
                )?);
                for vp in 1..self.options.cpucount {
                    self.directives.push(construct_ap_vmsa(
                        0xFFFFFFFFF000,
                        COMPATIBILITY_MASK,
                        self.options.platform,
                        self.firmware.get_fw_info().reset_addr,
                        vp,
                    )?);
                }
            }
            _ => (),
        }
        Ok(())
    }

    fn build_initialization(&mut self) -> Result<(), Box<dyn Error>> {
        let policy = match self.options.platform {
            cmd_options::Platform::Sev => 1,             // No Debug
            cmd_options::Platform::SevEs => 5,           // No Debug and ES required
            cmd_options::Platform::SevSnp => 0x30000u64, // Reserved bit set and SMT allowed
            cmd_options::Platform::Native => 0,
        };
        self.initialization
            .push(IgvmInitializationHeader::GuestPolicy {
                policy,
                compatibility_mask: COMPATIBILITY_MASK,
            });
        Ok(())
    }

    fn filter_pages(directive: &IgvmDirectiveHeader) -> bool {
        matches!(directive, IgvmDirectiveHeader::PageData { .. })
            || matches!(directive, IgvmDirectiveHeader::SnpVpContext { .. })
    }
}
