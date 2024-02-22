// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2024 SUSE LLC
//
// Author: Roy Hopkins <roy.hopkins@suse.com>

use std::error::Error;
use std::fs::File;
use std::io::Write;

use clap::Parser;
use igvm::{IgvmDirectiveHeader, IgvmFile, IgvmPlatformHeader, IgvmRevision};
use igvm_defs::{IgvmPlatformType, IGVM_VHS_SUPPORTED_PLATFORM};

use crate::cmd_options::CmdOptions;
use crate::ovmf_firmware::OvmfFirmware;
use crate::vmsa::{construct_ap_vmsa, construct_bsp_vmsa};

const COMPATIBILITY_MASK: u32 = 1;

pub struct IgvmBuilder {
    options: CmdOptions,
    firmware: OvmfFirmware,
    platforms: Vec<IgvmPlatformHeader>,
    directives: Vec<IgvmDirectiveHeader>,
}

impl IgvmBuilder {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let options = CmdOptions::parse();
        let firmware = OvmfFirmware::parse(&options.firmware, COMPATIBILITY_MASK)?;
        Ok(Self {
            options,
            firmware,
            platforms: vec![],
            directives: vec![],
        })
    }

    pub fn build(mut self) -> Result<(), Box<dyn Error>> {
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

        let file = IgvmFile::new(IgvmRevision::V1, self.platforms, vec![], self.directives)
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
        self.platforms.push(IgvmPlatformHeader::SupportedPlatform(
            IGVM_VHS_SUPPORTED_PLATFORM {
                compatibility_mask: COMPATIBILITY_MASK,
                highest_vtl: 0,
                platform_type: IgvmPlatformType::SEV_SNP,
                platform_version: 1,
                shared_gpa_boundary: 0,
            },
        ));
    }

    fn build_directives(&mut self) -> Result<(), Box<dyn Error>> {
        // Populate firmware directives.
        self.directives
            .extend_from_slice(self.firmware.directives());

        if let crate::cmd_options::Platform::SevEs = self.options.platform {
            // Build VMSAs for the required number of processors
            self.directives
                .push(construct_bsp_vmsa(0, COMPATIBILITY_MASK)?);
            for vp in 1..self.options.cpucount {
                self.directives.push(construct_ap_vmsa(
                    0,
                    COMPATIBILITY_MASK,
                    self.firmware.get_fw_info().reset_addr,
                    vp,
                )?);
            }
        }

        Ok(())
    }

    fn filter_pages(directive: &IgvmDirectiveHeader) -> bool {
        matches!(directive, IgvmDirectiveHeader::PageData { .. })
    }
}
