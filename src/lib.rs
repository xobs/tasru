//! Tasru: Parse Dwarf information from Elf files
//!
//! Tasru allows you to easily traverse Dwarf information stored within Elf files.
//! This can be used within a debugger to read complex data structures in a live
//! environment, or to perform forensics on a captured image.
//!
//! Example:
//!
//! ```no_run
//! /// Returns the address as a value, unless `resolve` is `false`
//! /// in which case it returns `0`. Useful for testing memory operations.
//! struct FakeReader {
//!     resolve: bool,
//! }
//!
//! impl tasru::memory::Read for FakeReader {
//!     type Error = std::io::Error; // Unused in this example
//!
//!     fn read_u8(&mut self, address: u64) -> Result<u8, Self::Error> {
//!         if self.resolve {
//!             Ok(address as u8 + 8)
//!         } else {
//!             Ok(0)
//!         }
//!     }
//! }
//!
//! // Read the elf file `example.elf`
//! let debug_info = tasru::DebugInfo::new(&"example.elf").expect("couldn't open example");
//! // Extract information on the static variable `example::ENUM`
//! let example_enum = debug_info.variable_from_demangled_name("example::ENUM").expect("couldn't find variable");
//! // Turn it into an enum (if it is one)
//! let example_enum = example_enum.enumeration().expect("variable isn't an enum");
//! // Get the current variant.
//! let variant = example_enum.variant(&mut FakeReader { resolve: true }).expect("couldn't determine variant");
//! println!("Variant is: {}", variant.name());
//! ```
//!
//! Most of the functionality in this crate comes from [`DebugInfo`].
pub mod debug_types;
mod dump;
pub mod extract;
pub mod memory;
pub mod unit_info;

use object::{Object, ObjectSection};
use std::borrow;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use debug_types::{DebugTypeError, DebugVariable};
use unit_info::UnitInfo;

pub(crate) type DwarfReaderLe = gimli::read::EndianRcSlice<gimli::LittleEndian>;
pub(crate) type DwarfReaderBe = gimli::read::EndianRcSlice<gimli::BigEndian>;

/// A collection of parsed Dwarf information for all compilation units within
/// the specified Elf file. This structure can be queried and will automatically
/// find links with all units.
pub struct DebugInfo {
    /// All the compilation units from within the Elf file
    units: Vec<UnitInfo>,
    /// A mapping from a particular [unit_info::DebugItemOffset](DebugItemOffset) to an address,
    /// useful for resolving a particular debug item to a given unit.
    symbol_unit_mapping: HashMap<unit_info::DebugItem, usize>,
}

#[derive(Debug)]
pub enum DebugInfoError {
    /// The .o file could not be parsed
    ObjectError(object::Error),
    /// An error occured reading the file off disk
    IoError(std::io::Error),
    /// An Elf format error occurred
    GimliError(gimli::Error),
    /// The requested variable could not be found
    VariableNotFound(String),
}

impl From<object::Error> for DebugInfoError {
    fn from(value: object::Error) -> Self {
        DebugInfoError::ObjectError(value)
    }
}

impl From<std::io::Error> for DebugInfoError {
    fn from(value: std::io::Error) -> Self {
        DebugInfoError::IoError(value)
    }
}

impl From<gimli::Error> for DebugInfoError {
    fn from(value: gimli::Error) -> Self {
        DebugInfoError::GimliError(value)
    }
}

impl core::fmt::Display for DebugInfoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebugInfoError::ObjectError(error) => write!(f, "Object Error: {}", error),
            DebugInfoError::IoError(error) => write!(f, "IO Error: {}", error),
            DebugInfoError::GimliError(error) => write!(f, "Gimli Error: {}", error),
            DebugInfoError::VariableNotFound(error) => write!(f, "Variable {} not found", error),
        }
    }
}

impl std::error::Error for DebugInfoError {}

impl DebugInfo {
    /// Create a new [DebugInfo] object from the Elf file pointed to at the specified file path.
    /// This will parse the file and extract each unit section, then perform a comprehensive parse
    /// of all symbols present within the file.
    pub fn new<P: AsRef<Path>>(file: &P) -> Result<DebugInfo, DebugInfoError> {
        let file = std::fs::read(file)?;
        let object = object::File::parse(file.as_slice())?;

        if object.is_little_endian() {
            Self::load_le(object)
        } else {
            Self::load_be(object)
        }
    }

    fn load_le(object: object::File<'_>) -> Result<DebugInfo, DebugInfoError> {
        let mut symbol_unit_mapping = HashMap::new();
        // Load a section and return as `Cow<[u8]>`.
        let load_section = |id: gimli::SectionId| -> Result<DwarfReaderLe, gimli::Error> {
            let data = object
                .section_by_name(id.name())
                .and_then(|section| section.uncompressed_data().ok())
                .unwrap_or_else(|| borrow::Cow::Borrowed(&[][..]));

            Ok(gimli::read::EndianRcSlice::new(
                Rc::from(&*data),
                gimli::LittleEndian,
            ))
        };

        // Load all of the sections.
        let dwarf_cow = gimli::Dwarf::load(&load_section)?;

        let mut units = Vec::new();
        let mut iter = dwarf_cow.units();

        let mut section_count = 0;
        while let Ok(Some(header)) = iter.next() {
            println!("Parsing unit {:?}", header);
            section_count += 1;
            if let Ok(unit) = dwarf_cow.unit(header) {
                // The DWARF V5 standard, section 2.4 specifies that the address size
                // for the object file (or the target architecture default) will be used for
                // DWARF debugging information.
                // The following line is a workaround for instances where the address size of the
                // CIE (Common Information Entry) is not correctly set.
                // The frame section address size is only used for CIE versions before 4.
                // frame_section.set_address_size(unit.encoding().address_size);

                if let Some(unit) = UnitInfo::new(unit, &dwarf_cow) {
                    for symbol in unit.all_symbols() {
                        assert!(symbol_unit_mapping.insert(symbol, units.len()).is_none());
                    }
                    units.push(unit);
                }
            } else {
                println!("Skipping unit");
            }
        }
        println!("Processed {} sections", section_count);

        Ok(DebugInfo {
            units,
            symbol_unit_mapping,
        })
    }

    fn load_be(object: object::File<'_>) -> Result<DebugInfo, DebugInfoError> {
        let mut symbol_unit_mapping = HashMap::new();
        // Load a section and return as `Cow<[u8]>`.
        let load_section = |id: gimli::SectionId| -> Result<DwarfReaderBe, gimli::Error> {
            let data = object
                .section_by_name(id.name())
                .and_then(|section| section.uncompressed_data().ok())
                .unwrap_or_else(|| borrow::Cow::Borrowed(&[][..]));

            Ok(gimli::read::EndianRcSlice::new(
                Rc::from(&*data),
                gimli::BigEndian,
            ))
        };

        // Load all of the sections.
        let dwarf_cow = gimli::Dwarf::load(&load_section)?;

        let mut units = Vec::new();
        let mut iter = dwarf_cow.units();

        let mut section_count = 0;
        while let Ok(Some(header)) = iter.next() {
            println!("Parsing unit {:?}", header);
            section_count += 1;
            if let Ok(unit) = dwarf_cow.unit(header) {
                // The DWARF V5 standard, section 2.4 specifies that the address size
                // for the object file (or the target architecture default) will be used for
                // DWARF debugging information.
                // The following line is a workaround for instances where the address size of the
                // CIE (Common Information Entry) is not correctly set.
                // The frame section address size is only used for CIE versions before 4.
                // frame_section.set_address_size(unit.encoding().address_size);

                if let Some(unit) = UnitInfo::new(unit, &dwarf_cow) {
                    for symbol in unit.all_symbols() {
                        assert!(symbol_unit_mapping.insert(symbol, units.len()).is_none());
                    }
                    units.push(unit);
                }
            } else {
                println!("Skipping unit");
            }
        }
        println!("Processed {} sections", section_count);

        Ok(DebugInfo {
            units,
            symbol_unit_mapping,
        })
    }

    /// Consult all units to look for a variant with the specified name. If the variable
    /// cannot be found, return an error. Note that only rustc name mangling is supported.
    pub fn variable_from_demangled_name(
        &self,
        path: &str,
    ) -> Result<DebugVariable, DebugTypeError> {
        for unit in &self.units {
            if let Some(variable) = unit.variable_from_demangled_name(path) {
                return Ok(DebugVariable::new(unit, self, variable));
            }
        }
        Err(DebugTypeError::VariableNotFound(path.into()))
    }

    /// Get the size of the specified debug item. Any debug item may be specified here,
    /// though some types may return `None` if their size couldn't be determined.
    pub fn size_from_item(&self, item: unit_info::DebugItem) -> Option<unit_info::StructOffset> {
        self.symbol_unit_mapping
            .get(&item)
            .map(|var| self.units[*var].size_from_item(item))
            .flatten()
    }

    /// Given an item, return the Variable object. If the item is not a Variable, or couldn't
    /// be located, return `None`.
    pub fn variable_from_item(&self, item: unit_info::DebugItem) -> Option<&unit_info::Variable> {
        self.symbol_unit_mapping
            .get(&item)
            .map(|var| self.units[*var].variable_from_item(item))
            .flatten()
    }

    /// Given an item, return the Structure object. If the item is not a Structure, or couldn't
    /// be located, return `None`.
    pub fn structure_from_item(&self, item: unit_info::DebugItem) -> Option<&unit_info::Structure> {
        self.symbol_unit_mapping
            .get(&item)
            .map(|var| self.units[*var].structure_from_item(item))
            .flatten()
    }

    /// Given an item, return the Enumeration object. If the item is not an Enumeration, or couldn't
    /// be located, return `None`.
    pub fn enumeration_from_item(
        &self,
        item: unit_info::DebugItem,
    ) -> Option<&unit_info::Enumeration> {
        self.symbol_unit_mapping
            .get(&item)
            .map(|var| self.units[*var].enumeration_from_item(item))
            .flatten()
    }

    /// Given an item, return the Pointer object. If the item is not a Pointer, or couldn't
    /// be located, return `None`.
    pub fn pointer_from_item(&self, item: unit_info::DebugItem) -> Option<&unit_info::Pointer> {
        self.symbol_unit_mapping
            .get(&item)
            .map(|var| self.units[*var].pointer_from_item(item))
            .flatten()
    }

    /// Given an item, return the Array object. If the item is not an Array, or couldn't
    /// be located, return `None`.
    pub fn array_from_item(&self, item: unit_info::DebugItem) -> Option<&unit_info::Array> {
        self.symbol_unit_mapping
            .get(&item)
            .map(|var| self.units[*var].array_from_item(item))
            .flatten()
    }

    /// Given an item, return the Union object. If the item is not a Union, or couldn't
    /// be located, return `None`.
    pub fn union_from_item(&self, item: unit_info::DebugItem) -> Option<&unit_info::Union> {
        self.symbol_unit_mapping
            .get(&item)
            .map(|var| self.units[*var].union_from_item(item))
            .flatten()
    }

    /// Given an item, return the BaseType object. If the item is not a BaseType, or couldn't
    /// be located, return `None`.
    pub fn base_type_from_item(&self, item: unit_info::DebugItem) -> Option<&unit_info::BaseType> {
        self.symbol_unit_mapping
            .get(&item)
            .map(|var| self.units[*var].base_type_from_item(item))
            .flatten()
    }
}
