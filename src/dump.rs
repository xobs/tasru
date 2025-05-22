#![allow(unused)]

use gimli::{EndianReader, Endianity, Reader, UnitOffset, UnitSectionOffset};
use std::rc::Rc;

fn dump_file_index<ENDIAN: Endianity>(
    file_index: u64,
    unit: gimli::UnitRef<gimli::EndianReader<ENDIAN, std::rc::Rc<[u8]>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if file_index == 0 && unit.header.version() <= 4 {
        return Ok(());
    }
    let header = match unit.line_program {
        Some(ref program) => program.header(),
        None => return Ok(()),
    };
    let file = match header.file(file_index) {
        Some(file) => file,
        None => {
            println!("Unable to get header for file {}", file_index);
            return Ok(());
        }
    };
    print!(" ");
    if let Some(directory) = file.directory(header) {
        let directory = unit.attr_string(directory)?;
        let directory = directory.to_string_lossy()?;
        if file.directory_index() != 0 && !directory.starts_with('/') {
            if let Some(ref comp_dir) = unit.comp_dir {
                print!("{}/", comp_dir.to_string_lossy()?,);
            }
        }
        print!("{}/", directory);
    }
    print!("{}", unit.attr_string(file.path_name())?.to_string_lossy()?);
    Ok(())
}

fn dump_range(range: Option<gimli::Range>) {
    if let Some(range) = range {
        print!(" [{:#x}, {:#x}]", range.begin, range.end);
    } else {
        print!(" [ignored]");
    }
}

fn dump_range_list<ENDIAN: Endianity>(
    offset: gimli::RangeListsOffset<<EndianReader<ENDIAN, Rc<[u8]>> as Reader>::Offset>,
    unit: gimli::UnitRef<gimli::EndianReader<ENDIAN, std::rc::Rc<[u8]>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut ranges = unit.ranges(offset)?;
    println!(
        "<rnglist at {}+0x{:08x}>",
        if unit.encoding().version < 5 {
            ".debug_ranges"
        } else {
            ".debug_rnglists"
        },
        offset.0,
    );
    let mut i = 0;
    while let Some(raw) = ranges.next_raw()? {
        print!("\t\t\t[{:2}] ", i);
        i += 1;
        let range = ranges.convert_raw(raw.clone())?;
        match raw {
            gimli::RawRngListEntry::BaseAddress { addr } => {
                println!("<new base address {:#x}>", addr);
            }
            gimli::RawRngListEntry::BaseAddressx { addr } => {
                let addr_val = unit.address(addr)?;
                println!("<new base addressx [{}]{:#x}>", addr.0, addr_val);
            }
            gimli::RawRngListEntry::StartxEndx { begin, end } => {
                let begin_val = unit.address(begin)?;
                let end_val = unit.address(end)?;
                print!(
                    "<startx-endx [{}]{:#x}, [{}]{:#x}>",
                    begin.0, begin_val, end.0, end_val,
                );
                dump_range(range);
                println!();
            }
            gimli::RawRngListEntry::StartxLength { begin, length } => {
                let begin_val = unit.address(begin)?;
                print!(
                    "<startx-length [{}]{:#x}, {:#x}>",
                    begin.0, begin_val, length,
                );
                dump_range(range);
                println!();
            }
            gimli::RawRngListEntry::AddressOrOffsetPair { begin, end }
            | gimli::RawRngListEntry::OffsetPair { begin, end } => {
                print!("<offset-pair {:#x}, {:#x}>", begin, end);
                dump_range(range);
                println!();
            }
            gimli::RawRngListEntry::StartEnd { begin, end } => {
                print!("<start-end {:#x}, {:#x}>", begin, end);
                dump_range(range);
                println!();
            }
            gimli::RawRngListEntry::StartLength { begin, length } => {
                print!("<start-length {:#x}, {:#x}>", begin, length);
                dump_range(range);
                println!();
            }
        };
    }
    Ok(())
}

fn dump_op<ENDIAN: Endianity>(
    unit: gimli::UnitRef<gimli::EndianReader<ENDIAN, std::rc::Rc<[u8]>>>,
    mut pc: gimli::EndianReader<ENDIAN, std::rc::Rc<[u8]>>,
    op: gimli::Operation<gimli::EndianReader<ENDIAN, std::rc::Rc<[u8]>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let dwop = gimli::DwOp(pc.read_u8()?);
    print!("{}", dwop);
    match op {
        gimli::Operation::Deref {
            base_type, size, ..
        } => {
            if dwop == gimli::DW_OP_deref_size || dwop == gimli::DW_OP_xderef_size {
                print!(" {}", size);
            }
            if base_type != UnitOffset(0) {
                print!(" type 0x{:08x}", base_type.0);
            }
        }
        gimli::Operation::Pick { index } => {
            if dwop == gimli::DW_OP_pick {
                print!(" {}", index);
            }
        }
        gimli::Operation::PlusConstant { value } => {
            print!(" {}", value as i64);
        }
        gimli::Operation::Bra { target } => {
            print!(" {}", target);
        }
        gimli::Operation::Skip { target } => {
            print!(" {}", target);
        }
        gimli::Operation::SignedConstant { value } => match dwop {
            gimli::DW_OP_const1s
            | gimli::DW_OP_const2s
            | gimli::DW_OP_const4s
            | gimli::DW_OP_const8s
            | gimli::DW_OP_consts => {
                print!(" {}", value);
            }
            _ => {}
        },
        gimli::Operation::UnsignedConstant { value } => match dwop {
            gimli::DW_OP_const1u
            | gimli::DW_OP_const2u
            | gimli::DW_OP_const4u
            | gimli::DW_OP_const8u
            | gimli::DW_OP_constu => {
                print!(" {}", value);
            }
            _ => {
                // These have the value encoded in the operation, eg DW_OP_lit0.
            }
        },
        gimli::Operation::Register { register } => {
            if dwop == gimli::DW_OP_regx {
                print!(" {}", register.0);
            }
        }
        gimli::Operation::RegisterOffset {
            register,
            offset,
            base_type,
        } => {
            if dwop >= gimli::DW_OP_breg0 && dwop <= gimli::DW_OP_breg31 {
                print!("{:+}", offset);
            } else {
                print!(" {}", register.0);
                if offset != 0 {
                    print!("{:+}", offset);
                }
                if base_type != UnitOffset(0) {
                    print!(" type 0x{:08x}", base_type.0);
                }
            }
        }
        gimli::Operation::FrameOffset { offset } => {
            print!(" {}", offset);
        }
        gimli::Operation::Call { offset } => match offset {
            gimli::DieReference::UnitRef(gimli::UnitOffset(offset)) => {
                print!(" 0x{:08x}", offset);
            }
            gimli::DieReference::DebugInfoRef(gimli::DebugInfoOffset(offset)) => {
                print!(" 0x{:08x}", offset);
            }
        },
        gimli::Operation::Piece {
            size_in_bits,
            bit_offset: None,
        } => {
            print!(" {}", size_in_bits / 8);
        }
        gimli::Operation::Piece {
            size_in_bits,
            bit_offset: Some(bit_offset),
        } => {
            print!(" 0x{:08x} offset 0x{:08x}", size_in_bits, bit_offset);
        }
        gimli::Operation::ImplicitValue { data } => {
            let data = data.to_slice()?;
            print!(" len {:#x} contents 0x", data.len());
            for byte in data.iter() {
                print!("{:02x}", byte);
            }
        }
        gimli::Operation::ImplicitPointer { value, byte_offset } => {
            print!(" 0x{:08x} {}", value.0, byte_offset);
        }
        gimli::Operation::EntryValue { expression } => {
            print!("(");
            dump_exprloc(unit, &gimli::Expression(expression))?;
            print!(")");
        }
        gimli::Operation::ParameterRef { offset } => {
            print!(" 0x{:08x}", offset.0);
        }
        gimli::Operation::Address { address } => {
            print!(" {:#x}", address);
        }
        gimli::Operation::AddressIndex { index } => {
            print!(" {:#x}", index.0);
            let address = unit.address(index)?;
            print!(" ({:#x})", address);
        }
        gimli::Operation::ConstantIndex { index } => {
            print!(" {:#x}", index.0);
            let address = unit.address(index)?;
            print!(" ({:#x})", address);
        }
        gimli::Operation::TypedLiteral { base_type, value } => {
            print!(" type 0x{:08x} contents 0x", base_type.0);
            for byte in value.to_slice()?.iter() {
                print!("{:02x}", byte);
            }
        }
        gimli::Operation::Convert { base_type } => {
            print!(" type 0x{:08x}", base_type.0);
        }
        gimli::Operation::Reinterpret { base_type } => {
            print!(" type 0x{:08x}", base_type.0);
        }
        gimli::Operation::WasmLocal { index }
        | gimli::Operation::WasmGlobal { index }
        | gimli::Operation::WasmStack { index } => {
            let wasmop = pc.read_u8()?;
            print!(" 0x{:x} 0x{:x}", wasmop, index);
        }
        gimli::Operation::Drop
        | gimli::Operation::Swap
        | gimli::Operation::Rot
        | gimli::Operation::Abs
        | gimli::Operation::And
        | gimli::Operation::Div
        | gimli::Operation::Minus
        | gimli::Operation::Mod
        | gimli::Operation::Mul
        | gimli::Operation::Neg
        | gimli::Operation::Not
        | gimli::Operation::Or
        | gimli::Operation::Plus
        | gimli::Operation::Shl
        | gimli::Operation::Shr
        | gimli::Operation::Shra
        | gimli::Operation::Xor
        | gimli::Operation::Eq
        | gimli::Operation::Ge
        | gimli::Operation::Gt
        | gimli::Operation::Le
        | gimli::Operation::Lt
        | gimli::Operation::Ne
        | gimli::Operation::Nop
        | gimli::Operation::PushObjectAddress
        | gimli::Operation::TLS
        | gimli::Operation::CallFrameCFA
        | gimli::Operation::StackValue => {}
    };
    Ok(())
}

fn dump_exprloc<ENDIAN: Endianity>(
    unit: gimli::UnitRef<gimli::EndianReader<ENDIAN, std::rc::Rc<[u8]>>>,
    data: &gimli::Expression<gimli::EndianReader<ENDIAN, std::rc::Rc<[u8]>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pc = data.0.clone();
    let mut space = false;
    while pc.len() != 0 {
        let pc_clone = pc.clone();
        match gimli::Operation::parse(&mut pc, unit.encoding()) {
            Ok(op) => {
                if space {
                    print!(" ");
                } else {
                    space = true;
                }
                dump_op(unit, pc_clone, op)?;
            }
            Err(gimli::Error::InvalidExpression(op)) => {
                println!("WARNING: unsupported operation 0x{:02x}", op.0);
                return Ok(());
            }
            Err(gimli::Error::UnsupportedRegister(register)) => {
                println!("WARNING: unsupported register {}", register);
                return Ok(());
            }
            Err(gimli::Error::UnexpectedEof(_)) => {
                println!("WARNING: truncated or malformed expression");
                return Ok(());
            }
            Err(e) => {
                println!("WARNING: unexpected operation parse error: {}", e);
                return Ok(());
            }
        }
    }
    Ok(())
}

fn dump_loc_list<ENDIAN: Endianity>(
    offset: gimli::LocationListsOffset<<EndianReader<ENDIAN, Rc<[u8]>> as Reader>::Offset>,
    unit: gimli::UnitRef<gimli::EndianReader<ENDIAN, std::rc::Rc<[u8]>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut locations = unit.locations(offset)?;
    println!(
        "<loclist at {}+0x{:08x}>",
        if unit.encoding().version < 5 {
            ".debug_loc"
        } else {
            ".debug_loclists"
        },
        offset.0,
    );
    let mut i = 0;
    while let Some(raw) = locations.next_raw()? {
        print!("\t\t\t[{:2}]", i);
        i += 1;
        let range = locations
            .convert_raw(raw.clone())?
            .map(|location| location.range);
        match raw {
            gimli::RawLocListEntry::BaseAddress { addr } => {
                println!("<base-address {:#x}>", addr);
            }
            gimli::RawLocListEntry::BaseAddressx { addr } => {
                let addr_val = unit.address(addr)?;
                println!("<base-addressx [{}]{:#x}>", addr.0, addr_val);
            }
            gimli::RawLocListEntry::StartxEndx {
                begin,
                end,
                ref data,
            } => {
                let begin_val = unit.address(begin)?;
                let end_val = unit.address(end)?;
                print!(
                    "<startx-endx [{}]{:#x}, [{}]{:#x}>",
                    begin.0, begin_val, end.0, end_val,
                );
                dump_range(range);
                dump_exprloc(unit, data)?;
                println!();
            }
            gimli::RawLocListEntry::StartxLength {
                begin,
                length,
                ref data,
            } => {
                let begin_val = unit.address(begin)?;
                print!(
                    "<startx-length [{}]{:#x}, {:#x}>",
                    begin.0, begin_val, length,
                );
                dump_range(range);
                dump_exprloc(unit, data)?;
                println!();
            }
            gimli::RawLocListEntry::AddressOrOffsetPair {
                begin,
                end,
                ref data,
            }
            | gimli::RawLocListEntry::OffsetPair {
                begin,
                end,
                ref data,
            } => {
                print!("<offset-pair {:#x}, {:#x}>", begin, end);
                dump_range(range);
                dump_exprloc(unit, data)?;
                println!();
            }
            gimli::RawLocListEntry::DefaultLocation { ref data } => {
                print!("<default location>");
                dump_exprloc(unit, data)?;
                println!();
            }
            gimli::RawLocListEntry::StartEnd {
                begin,
                end,
                ref data,
            } => {
                print!("<start-end {:#x}, {:#x}>", begin, end);
                dump_range(range);
                dump_exprloc(unit, data)?;
                println!();
            }
            gimli::RawLocListEntry::StartLength {
                begin,
                length,
                ref data,
            } => {
                print!("<start-length {:#x}, {:#x}>", begin, length);
                dump_range(range);
                dump_exprloc(unit, data)?;
                println!();
            }
        };
    }
    Ok(())
}

pub fn attribute<ENDIAN: Endianity>(
    attr: &gimli::Attribute<gimli::EndianReader<ENDIAN, std::rc::Rc<[u8]>>>,
    unit: gimli::UnitRef<gimli::EndianReader<ENDIAN, std::rc::Rc<[u8]>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = attr.value();
    match value {
        gimli::AttributeValue::Addr(address) => {
            println!("{:#x}", address);
        }
        gimli::AttributeValue::Block(data) => {
            for byte in data.iter() {
                print!("{:02x}", byte);
            }
            println!();
        }
        gimli::AttributeValue::Data1(_)
        | gimli::AttributeValue::Data2(_)
        | gimli::AttributeValue::Data4(_)
        | gimli::AttributeValue::Data8(_) => {
            if let (Some(udata), Some(sdata)) = (attr.udata_value(), attr.sdata_value()) {
                if sdata >= 0 {
                    println!("{}", udata);
                } else {
                    println!("{} ({})", udata, sdata);
                }
            } else {
                println!("{:?}", value);
            }
        }
        gimli::AttributeValue::Sdata(data) => {
            match attr.name() {
                gimli::DW_AT_data_member_location => {
                    println!("{}", data);
                }
                _ => {
                    if data >= 0 {
                        println!("0x{:08x}", data);
                    } else {
                        println!("0x{:08x} ({})", data, data);
                    }
                }
            };
        }
        gimli::AttributeValue::Udata(data) => {
            match attr.name() {
                gimli::DW_AT_high_pc => {
                    println!("<offset-from-lowpc>{}", data);
                }
                gimli::DW_AT_data_member_location => {
                    if let Some(sdata) = attr.sdata_value() {
                        // This is a DW_FORM_data* value.
                        // libdwarf-dwarfdump displays this as signed too.
                        if sdata >= 0 {
                            println!("{}", data);
                        } else {
                            println!("{} ({})", data, sdata);
                        }
                    } else {
                        println!("{}", data);
                    }
                }
                gimli::DW_AT_lower_bound | gimli::DW_AT_upper_bound => {
                    println!("{}", data);
                }
                _ => {
                    println!("0x{:08x}", data);
                }
            };
        }
        gimli::AttributeValue::Exprloc(ref data) => {
            if let gimli::AttributeValue::Exprloc(_) = attr.raw_value() {
                print!("len 0x{:04x}: ", data.0.len());
                for byte in data.0.iter() {
                    print!("{:02x}", byte);
                }
                print!(": ");
            }
            dump_exprloc(unit, data)?;
            println!();
        }
        gimli::AttributeValue::Flag(true) => {
            println!("yes");
        }
        gimli::AttributeValue::Flag(false) => {
            println!("no");
        }
        gimli::AttributeValue::SecOffset(offset) => {
            println!("0x{:08x}", offset);
        }
        gimli::AttributeValue::DebugAddrBase(base) => {
            println!("<.debug_addr+0x{:08x}>", base.0);
        }
        gimli::AttributeValue::DebugAddrIndex(index) => {
            print!("(index {:#x}): ", index.0);
            let address = unit.address(index)?;
            println!("{:#x}", address);
        }
        gimli::AttributeValue::UnitRef(offset) => {
            print!("0x{:08x}", offset.0);
            match offset.to_unit_section_offset(&unit) {
                UnitSectionOffset::DebugInfoOffset(goff) => {
                    print!("<.debug_info+0x{:08x}>", goff.0);
                }
                UnitSectionOffset::DebugTypesOffset(goff) => {
                    print!("<.debug_types+0x{:08x}>", goff.0);
                }
            }
            println!();
        }
        gimli::AttributeValue::DebugInfoRef(offset) => {
            println!("<.debug_info+0x{:08x}>", offset.0);
        }
        gimli::AttributeValue::DebugInfoRefSup(offset) => {
            println!("<.debug_info(sup)+0x{:08x}>", offset.0);
        }
        gimli::AttributeValue::DebugLineRef(offset) => {
            println!("<.debug_line+0x{:08x}>", offset.0);
        }
        gimli::AttributeValue::LocationListsRef(offset) => {
            dump_loc_list(offset, unit)?;
        }
        gimli::AttributeValue::DebugLocListsBase(base) => {
            println!("<.debug_loclists+0x{:08x}>", base.0);
        }
        gimli::AttributeValue::DebugLocListsIndex(index) => {
            print!("(indirect location list, index {:#x}): ", index.0);
            let offset = unit.locations_offset(index)?;
            dump_loc_list(offset, unit)?;
        }
        gimli::AttributeValue::DebugMacinfoRef(offset) => {
            println!("<.debug_macinfo+0x{:08x}>", offset.0);
        }
        gimli::AttributeValue::DebugMacroRef(offset) => {
            println!("<.debug_macro+0x{:08x}>", offset.0);
        }
        gimli::AttributeValue::RangeListsRef(offset) => {
            let offset = unit.ranges_offset_from_raw(offset);
            dump_range_list(offset, unit)?;
        }
        gimli::AttributeValue::DebugRngListsBase(base) => {
            println!("<.debug_rnglists+0x{:08x}>", base.0);
        }
        gimli::AttributeValue::DebugRngListsIndex(index) => {
            print!("(indirect range list, index {:#x}): ", index.0);
            let offset = unit.ranges_offset(index)?;
            dump_range_list(offset, unit)?;
        }
        gimli::AttributeValue::DebugTypesRef(signature) => {
            print!("0x{:016x}", signature.0);
            println!(" <type signature>");
        }
        gimli::AttributeValue::DebugStrRef(offset) => {
            if let Ok(s) = unit.string(offset) {
                println!("{}", s.to_string_lossy()?);
            } else {
                println!("<.debug_str+0x{:08x}>", offset.0);
            }
        }
        gimli::AttributeValue::DebugStrRefSup(offset) => {
            if let Ok(s) = unit.sup_string(offset) {
                println!("{}", s.to_string_lossy()?);
            } else {
                println!("<.debug_str(sup)+0x{:08x}>", offset.0);
            }
        }
        gimli::AttributeValue::DebugStrOffsetsBase(base) => {
            println!("<.debug_str_offsets+0x{:08x}>", base.0);
        }
        gimli::AttributeValue::DebugStrOffsetsIndex(index) => {
            print!("(indirect string, index {:#x}): ", index.0);
            let offset = unit.string_offset(index)?;
            if let Ok(s) = unit.string(offset) {
                println!("{}", s.to_string_lossy()?);
            } else {
                println!("<.debug_str+0x{:08x}>", offset.0);
            }
        }
        gimli::AttributeValue::DebugLineStrRef(offset) => {
            if let Ok(s) = unit.line_string(offset) {
                println!("{}", s.to_string_lossy()?);
            } else {
                println!("<.debug_line_str=0x{:08x}>", offset.0);
            }
        }
        gimli::AttributeValue::String(s) => {
            println!("{}", s.to_string_lossy()?);
        }
        gimli::AttributeValue::Encoding(value) => {
            println!("{}", value);
        }
        gimli::AttributeValue::DecimalSign(value) => {
            println!("{}", value);
        }
        gimli::AttributeValue::Endianity(value) => {
            println!("{}", value);
        }
        gimli::AttributeValue::Accessibility(value) => {
            println!("{}", value);
        }
        gimli::AttributeValue::Visibility(value) => {
            println!("{}", value);
        }
        gimli::AttributeValue::Virtuality(value) => {
            println!("{}", value);
        }
        gimli::AttributeValue::Language(value) => {
            println!("{}", value);
        }
        gimli::AttributeValue::AddressClass(value) => {
            println!("{}", value);
        }
        gimli::AttributeValue::IdentifierCase(value) => {
            println!("{}", value);
        }
        gimli::AttributeValue::CallingConvention(value) => {
            println!("{}", value);
        }
        gimli::AttributeValue::Inline(value) => {
            println!("{}", value);
        }
        gimli::AttributeValue::Ordering(value) => {
            println!("{}", value);
        }
        gimli::AttributeValue::FileIndex(value) => {
            print!("0x{:08x}", value);
            dump_file_index(value, unit)?;
            println!();
        }
        gimli::AttributeValue::DwoId(value) => {
            println!("0x{:016x}", value.0);
        }
    }

    Ok(())
}

fn spaces(buf: &mut String, len: usize) -> &str {
    while buf.len() < len {
        buf.push(' ');
    }
    &buf[..len]
}

pub fn abbreviation<ENDIAN: Endianity>(
    unit: &gimli::UnitRef<gimli::EndianReader<ENDIAN, std::rc::Rc<[u8]>>>,
    entries: &mut gimli::EntriesRaw<gimli::EndianReader<ENDIAN, std::rc::Rc<[u8]>>>,
    abbreviation: &gimli::Abbreviation,
    indent: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut spaces_buf = String::new();
    for spec in abbreviation.attributes() {
        let attr = entries.read_attribute(*spec)?;
        print!("{}", spaces(&mut spaces_buf, indent));
        if let Some(n) = attr.name().static_string() {
            let right_padding = 27 - 27.min(n.len());
            print!("{}{} ", n, spaces(&mut spaces_buf, right_padding));
        } else {
            print!("{:27} ", attr.name());
        }
        if let Err(e) = attribute(&attr, *unit) {
            eprintln!("Failed to dump attribute value: {}", e);
        }
    }

    Ok(())
}

#[allow(unused)]
pub fn unit_ref<ENDIAN: Endianity>(
    unit: gimli::UnitRef<gimli::EndianReader<ENDIAN, std::rc::Rc<[u8]>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut spaces_buf = String::new();

    let mut entries = unit.entries_raw(None)?;
    while !entries.is_empty() {
        let offset = entries.next_offset();
        let depth = entries.next_depth();
        let abbrev = entries.read_abbreviation()?;

        let mut indent = if depth >= 0 {
            depth as usize * 2 + 2
        } else {
            2
        };
        print!("<{}{}>", if depth < 10 { " " } else { "" }, depth);
        print!("<0x{:08x}>", offset.0);
        println!(
            "{}{}",
            spaces(&mut spaces_buf, indent),
            abbrev.map(|x| x.tag()).unwrap_or(gimli::DW_TAG_null)
        );

        indent += 18;

        for spec in abbrev.map(|x| x.attributes()).unwrap_or(&[]) {
            let attr = entries.read_attribute(*spec)?;
            print!("{}", spaces(&mut spaces_buf, indent));
            if let Some(n) = attr.name().static_string() {
                let right_padding = 27 - 27.min(n.len());
                print!("{}{} ", n, spaces(&mut spaces_buf, right_padding));
            } else {
                print!("{:27} ", attr.name());
            }
            if let Err(e) = attribute(&attr, unit) {
                eprintln!("Failed to dump attribute value: {}", e);
            }
        }
    }
    Ok(())
}
