// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2024 SUSE LLC
//
// Author: Roy Hopkins <roy.hopkins@suse.com>

use std::error::Error;

use igvm::snp_defs::SevVmsa;
use igvm::IgvmDirectiveHeader;
use zerocopy::FromZeroes;

pub fn construct_ap_vmsa(
    gpa_start: u64,
    compatibility_mask: u32,
    reset_addr: u32,
    vp_index: u16,
) -> Result<IgvmDirectiveHeader, Box<dyn Error>> {
    let mut vmsa_box = SevVmsa::new_box_zeroed();
    let vmsa = vmsa_box.as_mut();

    // Establish CS as a 32-bit code selector.
    vmsa.cs.selector = 0xf000;
    vmsa.cs.base = reset_addr as u64 & 0xffff0000;
    vmsa.cs.limit = 0xffff;
    vmsa.cs.attrib = 0x9b;

    vmsa.ds.selector = 0;
    vmsa.ds.base = 0;
    vmsa.ds.limit = 0xffff;
    vmsa.ds.attrib = 0x93;
    vmsa.es.selector = 0;
    vmsa.es.base = 0;
    vmsa.es.limit = 0xffff;
    vmsa.es.attrib = 0x93;
    vmsa.fs.selector = 0;
    vmsa.fs.base = 0;
    vmsa.fs.limit = 0xffff;
    vmsa.fs.attrib = 0x93;
    vmsa.gs.selector = 0;
    vmsa.gs.base = 0;
    vmsa.gs.limit = 0xffff;
    vmsa.gs.attrib = 0x93;
    vmsa.ss.selector = 0;
    vmsa.ss.base = 0;
    vmsa.ss.limit = 0xffff;
    vmsa.ss.attrib = 0x93;

    vmsa.cr0 = 0x60000010;
    vmsa.xcr0 = 1;

    vmsa.rip = reset_addr as u64 & 0xffff;

    Ok(IgvmDirectiveHeader::SnpVpContext {
        gpa: gpa_start,
        compatibility_mask,
        vp_index,
        vmsa: vmsa_box,
    })
}

pub fn construct_bsp_vmsa(
    gpa_start: u64,
    compatibility_mask: u32,
) -> Result<IgvmDirectiveHeader, Box<dyn Error>> {
    let mut vmsa_box = SevVmsa::new_box_zeroed();
    let vmsa = vmsa_box.as_mut();

    let reset_addr = 0xfffffff0u32;

    // Establish CS as a 32-bit code selector.
    vmsa.cs.selector = 0xf000;
    vmsa.cs.base = reset_addr as u64 & 0xffff0000;
    vmsa.cs.limit = 0xffff;
    vmsa.cs.attrib = 0x9b;

    vmsa.ds.selector = 0;
    vmsa.ds.base = 0;
    vmsa.ds.limit = 0xffff;
    vmsa.ds.attrib = 0x93;
    vmsa.es.selector = 0;
    vmsa.es.base = 0;
    vmsa.es.limit = 0xffff;
    vmsa.es.attrib = 0x93;
    vmsa.fs.selector = 0;
    vmsa.fs.base = 0;
    vmsa.fs.limit = 0xffff;
    vmsa.fs.attrib = 0x93;
    vmsa.gs.selector = 0;
    vmsa.gs.base = 0;
    vmsa.gs.limit = 0xffff;
    vmsa.gs.attrib = 0x93;
    vmsa.ss.selector = 0;
    vmsa.ss.base = 0;
    vmsa.ss.limit = 0xffff;
    vmsa.ss.attrib = 0x93;

    vmsa.cr0 = 0x60000010;
    vmsa.xcr0 = 1;

    vmsa.rip = reset_addr as u64 & 0xffff;

    Ok(IgvmDirectiveHeader::SnpVpContext {
        gpa: gpa_start,
        compatibility_mask,
        vp_index: 0,
        vmsa: vmsa_box,
    })
}
