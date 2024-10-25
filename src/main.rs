use encoding_rs::SHIFT_JIS;
use std::{io, fs, path::Path, error::Error};
use byteorder::{BigEndian, ByteOrder};
use csv;

const GIMMICK_TABLE_START: usize  = 0x8083B468;
const GIMMICK_TABLE_END: usize    = 0x8083CF98;
const GIMMICK_TABLE_SIZE: usize   = 0x10;
const NUM_TABLE_ENTRIES: usize    = (GIMMICK_TABLE_END - GIMMICK_TABLE_START) / GIMMICK_TABLE_SIZE;
const MEM8_BEGIN: usize = 0x80000000;

fn mem8_to_file(memory_address: usize) -> usize {
    memory_address - MEM8_BEGIN
}

fn shift_jis_to_utf8(raw_bytes: Vec<u8>) -> String {
    let (decoded, _, has_errors) = SHIFT_JIS.decode(&raw_bytes);

    if has_errors {
        // decoding error for whatever reason
        return "<DECODE_ERROR>".to_string();
    }
    
    decoded.to_string()
}

// struct GimmickTableData {
//     description_address: u32,
//     resource_name_address: u32,
//     build_function_address: u32,
//     is_common: bool,
// }

struct GimmickTable {
    description: String,
    resource_name: String,
    build_function_address: u32,
    is_common: bool,
}


// for the purpose of this program:
// "offset" is relative to the ram dump
// "address" is relative to the game's memory

fn main() -> Result<(), Box<dyn Error>> {
    println!("Enter path to MEM8 RAM dump from Dolphin:");
    
    let mut filepath = String::new();

    io::stdin().read_line(&mut filepath)?;

    filepath = filepath.trim().to_string();

    let filepath = Path::new(&filepath);

    if !std::path::Path::exists(filepath) {
        return Err("file does not exist".into());
    }

    let ram_dump = fs::read(filepath.to_str().unwrap())?;
    
    let mut tables: Vec<GimmickTable> = Vec::new();

    for index in 0..NUM_TABLE_ENTRIES {
        let table_offset = mem8_to_file(GIMMICK_TABLE_START) + (index * GIMMICK_TABLE_SIZE);
        let table_contents = &ram_dump[table_offset..table_offset + GIMMICK_TABLE_SIZE];
        
        let mut table = GimmickTable {
            description: String::new(),
            resource_name: String::new(),
            build_function_address: 0,
            is_common: false
        };
        
        // 1/4 - read description
        let address = &table_contents[0..4];
        
        let offset = BigEndian::read_u32(address) as usize;
        
        if offset != 0 {
            let offset = mem8_to_file(offset);
            
            // read shiftjis bytes from ram_dump[offset] until zero
    
            let mut shift_jis_bytes: Vec<u8> = Vec::new();
    
            for &byte in &ram_dump[offset..] {
                if byte == 0x00 {
                    break;
                }
    
                shift_jis_bytes.push(byte);
            }
    
            table.description = shift_jis_to_utf8(shift_jis_bytes);
        }

        // 2/4 - read resource name
        let address = &table_contents[4..8];
        let offset = BigEndian::read_u32(address) as usize;

        if offset != 0 {
            let offset = mem8_to_file(offset);

            // the resource name is always valid ASCII

            let mut resource_name_bytes: Vec<u8> = Vec::new();

            for &byte in &ram_dump[offset..] {
                if byte == 0x00 {
                    break;
                }

                resource_name_bytes.push(byte);
            }

            table.resource_name = String::from_utf8(resource_name_bytes)?;
        }


        // 3/4 - get build function address
        let address = &table_contents[8..0xC];
        table.build_function_address = BigEndian::read_u32(address);

        // 4/4 - set "is common"
        table.is_common = table_contents.get(0xC).copied().unwrap_or(0) != 0;

        tables.push(table);
    }

    // write data to a csv with the following fields:

    // Gimmick ID (hex), Name, Resource Name, Is Common

    let csv_file = fs::File::create("gimmicks.csv")?;
    let mut writer = csv::Writer::from_writer(csv_file);

    writer.write_record(&["Gimmick ID", "Name", "Resource Name", "Build Function Address", "Common?"])?;

    for (index, table) in tables.iter().enumerate() {
        
        let description = {
            if table.description.is_empty() {
                String::from("<none>")
            } else {
                table.description.clone()
            }
        };

        let resource_name = {
            if table.resource_name.is_empty() {
                String::from("<none>")
            } else {
                table.resource_name.clone()
            }
        };

        let build_func_address = {
            if table.build_function_address == 0 {
                String::from("<none>")
            } else {
                format!("0x{:X}", table.build_function_address)
            }
        };

        writer.write_record(&[
            format!("0x{:X}", index),
            description,
            resource_name,
            build_func_address,
            table.is_common.to_string(),
        ])?;
    }

    writer.flush()?;

    
    println!("CSV written successfully.");
    Ok(())
}
