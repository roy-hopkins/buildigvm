// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2024 SUSE LLC
//
// Author: Roy Hopkins <roy.hopkins@suse.com>

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::mem::size_of;

use igvm::IgvmDirectiveHeader;
use igvm_defs::{IgvmPageDataFlags, IgvmPageDataType, PAGE_SIZE_4K};
use uuid::{uuid, Uuid};

use crate::cmd_options::Platform;

const OVMF_TABLE_FOOTER_GUID: Uuid = uuid!("96b582de-1fb2-45f7-baea-a366c55a082d");
const OVMF_SEV_METADATA_GUID: Uuid = uuid!("dc886566-984a-4798-a75e-5585a7bf67cc");
const SEV_INFO_BLOCK_GUID: Uuid = uuid!("00f771de-1a7e-4fcb-890e-68c77e2fb44e");

const SEV_META_DESC_TYPE_MEM: u32 = 1;
const SEV_META_DESC_TYPE_SECRETS: u32 = 2;
const SEV_META_DESC_TYPE_CPUID: u32 = 3;
const SEV_META_DESC_TYPE_CAA: u32 = 4;

// Offset from the end of the file where the OVMF table footer GUID should be.
const FOOTER_OFFSET: usize = 32;

/// An entry that represents an area of pre-validated memory defined by the
/// firmware in the IGVM file.
#[derive(Clone, Copy, Debug, Default)]
pub struct OvmfFwMem {
    /// The base physical address of the prevalidated memory region.
    pub base: u32,

    /// The length of the prevalidated memory region in bytes.
    pub size: u32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct OvmfFwInfo {
    /// The guest physical address of the start of the guest firmware. The
    /// permissions on the pages in the firmware range are adjusted to the guest
    /// VMPL. If this field is zero then no firmware is launched after
    /// initialization is complete.
    pub start: u32,

    /// The size of the guest firmware in bytes. If the firmware size is zero then
    /// no firmware is launched after initialization is complete.
    pub size: u32,

    /// The guest physical address at which the firmware expects to find the
    /// secrets page.
    pub secrets_page: u32,

    /// The guest physical address at which the firmware expects to find the
    /// calling area page.
    pub caa_page: u32,

    /// The guest physical address at which the firmware expects to find the
    /// CPUID page.
    pub cpuid_page: u32,

    /// The SEV-ES reset vector for APs
    pub reset_addr: u32,

    /// The number of prevalidated memory regions defined by the firmware.
    pub prevalidated_count: u32,

    /// The prevalidated memory regions defined by the firmware.
    pub prevalidated: [OvmfFwMem; 8],
}

struct MetadataDesc {
    pub base: u32,
    pub len: u32,
    pub metadata_type: u32,
}

impl MetadataDesc {
    pub fn size() -> usize {
        size_of::<u32>() * 3
    }
}

impl TryFrom<&[u8]> for MetadataDesc {
    type Error = Box<dyn Error>;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < Self::size() {
            return Err("Cannot parse OVMF metadata descriptor - invalid buffer size".into());
        }
        Ok(Self {
            base: read_u32(&value[0..4])?,
            len: read_u32(&value[4..8])?,
            metadata_type: read_u32(&value[8..12])?,
        })
    }
}

struct SevMetadata {
    pub _sig: u32,
    pub _len: u32,
    pub _version: u32,
    pub num_desc: u32,
}

impl SevMetadata {
    pub fn size() -> usize {
        size_of::<u32>() * 4
    }
}

impl TryFrom<&[u8]> for SevMetadata {
    type Error = Box<dyn Error>;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < Self::size() {
            return Err("Cannot parse OVMF metadata - invalid buffer size".into());
        }
        Ok(Self {
            _sig: read_u32(&value[0..4])?,
            _len: read_u32(&value[4..8])?,
            _version: read_u32(&value[8..12])?,
            num_desc: read_u32(&value[12..16])?,
        })
    }
}

struct TableInfo {
    uuid: Vec<u8>,
    data_offset: usize,
    data_length: u16,
}

fn read_u32(data: &[u8]) -> Result<u32, Box<dyn Error>> {
    if data.len() < 4 {
        Err("Invalid buffer passed to read_u32".into())
    } else {
        Ok(data[0] as u32
            + ((data[1] as u32) << 8)
            + ((data[2] as u32) << 16)
            + ((data[3] as u32) << 24))
    }
}

fn read_u16(data: &[u8]) -> Result<u16, Box<dyn Error>> {
    if data.len() < 2 {
        Err("Invalid buffer passed to read_u16".into())
    } else {
        Ok(data[0] as u16 + ((data[1] as u16) << 8))
    }
}

fn read_table(current_offset: usize, data: &[u8]) -> Result<TableInfo, Box<dyn Error>> {
    let uuid_size = size_of::<Uuid>();
    // current_offset is at the top of the structure.
    if current_offset < (uuid_size + 2) {
        return Err("Invalid metadata table in OVMF firmware".into());
    }
    let entry_uuid = data[current_offset - uuid_size..current_offset].to_vec();
    let table_size_offset = current_offset - uuid_size - 2;
    let table_size = read_u16(&data[table_size_offset..table_size_offset + 2])? as usize;
    if table_size > current_offset {
        return Err("Invalid metadata table in OVMF firmware".into());
    }
    Ok(TableInfo {
        uuid: entry_uuid,
        data_offset: current_offset - table_size,
        data_length: (table_size - uuid_size - 2) as u16,
    })
}

fn parse_sev_metadata(
    data: &[u8],
    table_data_offset: usize,
    firmware: &mut OvmfFwInfo,
) -> Result<(), Box<dyn Error>> {
    let offset = data.len() - read_u32(&data[table_data_offset..table_data_offset + 4])? as usize;
    let metadata = SevMetadata::try_from(&data[offset..offset + SevMetadata::size()])?;

    for i in 0..metadata.num_desc as usize {
        let desc_offset = offset + SevMetadata::size() + i * MetadataDesc::size();
        let metadata_desc =
            MetadataDesc::try_from(&data[desc_offset..desc_offset + MetadataDesc::size()])?;
        match metadata_desc.metadata_type {
            SEV_META_DESC_TYPE_MEM => {
                if firmware.prevalidated_count as usize == firmware.prevalidated.len() {
                    return Err("OVMF metadata defines too many memory regions".into());
                }
                firmware.prevalidated[firmware.prevalidated_count as usize].base =
                    metadata_desc.base;
                firmware.prevalidated[firmware.prevalidated_count as usize].size =
                    metadata_desc.len;
                firmware.prevalidated_count += 1;
            }
            SEV_META_DESC_TYPE_SECRETS => firmware.secrets_page = metadata_desc.base,
            SEV_META_DESC_TYPE_CPUID => firmware.cpuid_page = metadata_desc.base,
            SEV_META_DESC_TYPE_CAA => firmware.caa_page = metadata_desc.base,
            _ => {}
        }
    }

    Ok(())
}

fn parse_sev_info_block(data: &[u8], firmware: &mut OvmfFwInfo) -> Result<(), Box<dyn Error>> {
    // Not currently used
    firmware.reset_addr = read_u32(&data[0..4])?;
    Ok(())
}

fn parse_inner_table(
    current_offset: usize,
    data: &[u8],
    firmware: &mut OvmfFwInfo,
) -> Result<usize, Box<dyn Error>> {
    let table = read_table(current_offset, data)?;

    if table.uuid == OVMF_SEV_METADATA_GUID.to_bytes_le() {
        parse_sev_metadata(data, table.data_offset, firmware)?;
    } else if table.uuid == SEV_INFO_BLOCK_GUID.to_bytes_le() {
        parse_sev_info_block(
            &data[table.data_offset..table.data_offset + table.data_length as usize],
            firmware,
        )?;
    }

    Ok(table.data_offset)
}

pub fn parse_ovmf(data: &[u8], firmware: &mut OvmfFwInfo) -> Result<(), Box<dyn Error>> {
    // The OVMF metadata UUID is stored at a specific offset from the end of the file.
    let mut current_offset = data
        .len()
        .checked_sub(FOOTER_OFFSET)
        .ok_or("OVMF firmware file is too small")?;
    let ovmf_table = read_table(current_offset, data)?;
    if ovmf_table.uuid != OVMF_TABLE_FOOTER_GUID.to_bytes_le() {
        return Err("OVMF table footer not found".into());
    }
    current_offset = ovmf_table.data_offset + ovmf_table.data_length as usize;

    while current_offset > ovmf_table.data_offset {
        current_offset = parse_inner_table(current_offset, data, firmware)?;
    }

    Ok(())
}

pub struct OvmfFirmware {
    fw_info: OvmfFwInfo,
    directives: Vec<IgvmDirectiveHeader>,
}

impl OvmfFirmware {
    pub fn parse(
        filename: &String,
        compatibility_mask: u32,
        platform: Platform,
    ) -> Result<Self, Box<dyn Error>> {
        let mut in_file = File::open(filename).map_err(|e| {
            eprintln!("Failed to open firmware file {}", filename);
            e
        })?;
        let len = in_file.metadata()?.len() as usize;
        if len > 0xffffffff {
            return Err("OVMF firmware is too large".into());
        }
        let mut data = Vec::with_capacity(len);
        if in_file.read_to_end(&mut data)? != len {
            return Err("Failed to read OVMF file".into());
        }
        let mut fw_info = OvmfFwInfo::default();
        parse_ovmf(&data, &mut fw_info)?;

        // OVMF must be located to end at 4GB.
        fw_info.start = (0xffffffff - len + 1) as u32;
        fw_info.size = len as u32;

        // Build page directives for the file contents.
        let mut gpa: u64 = fw_info.start.into();
        let mut directives = Vec::<IgvmDirectiveHeader>::new();
        for page_data in data.chunks(PAGE_SIZE_4K as usize) {
            directives.push(IgvmDirectiveHeader::PageData {
                gpa,
                compatibility_mask,
                flags: IgvmPageDataFlags::new(),
                data_type: IgvmPageDataType::NORMAL,
                data: page_data.to_vec(),
            });
            gpa += PAGE_SIZE_4K;
        }

        if let Platform::SevSnp = platform {
            // Build page directives for the metadata
            directives.push(IgvmDirectiveHeader::PageData {
                gpa: fw_info.secrets_page as u64,
                compatibility_mask,
                flags: IgvmPageDataFlags::new(),
                data_type: IgvmPageDataType::SECRETS,
                data: vec![],
            });
            directives.push(IgvmDirectiveHeader::PageData {
                gpa: fw_info.caa_page as u64,
                compatibility_mask,
                flags: IgvmPageDataFlags::new(),
                data_type: IgvmPageDataType::NORMAL,
                data: vec![],
            });
            directives.push(IgvmDirectiveHeader::PageData {
                gpa: fw_info.cpuid_page as u64,
                compatibility_mask,
                flags: IgvmPageDataFlags::new(),
                data_type: IgvmPageDataType::CPUID_DATA,
                data: vec![],
            });
            for i in 0..fw_info.prevalidated_count {
                let pv_mem = fw_info.prevalidated[i as usize];
                for offset in (0..pv_mem.size).step_by(PAGE_SIZE_4K as usize) {
                    directives.push(IgvmDirectiveHeader::PageData {
                        gpa: (pv_mem.base + offset) as u64,
                        compatibility_mask,
                        flags: IgvmPageDataFlags::new(),
                        data_type: IgvmPageDataType::NORMAL,
                        data: vec![],
                    });
                }
            }
        }

        Ok(Self {
            fw_info,
            directives,
        })
    }

    pub fn directives(&self) -> &Vec<IgvmDirectiveHeader> {
        &self.directives
    }

    pub fn get_fw_info(&self) -> OvmfFwInfo {
        self.fw_info
    }
}
