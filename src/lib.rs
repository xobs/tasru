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

pub(crate) type GimliReader = gimli::EndianReader<gimli::LittleEndian, std::rc::Rc<[u8]>>;
pub(crate) type DwarfReader = gimli::read::EndianRcSlice<gimli::LittleEndian>;

pub struct DebugInfo {
    units: Vec<UnitInfo>,
    symbol_unit_mapping: HashMap<unit_info::DebugItemOffset, usize>,
}

#[derive(Debug)]
pub enum DebugInfoError {
    ObjectError(object::Error),
    IoError(std::io::Error),
    GimliError(gimli::Error),
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
    pub fn new<P: AsRef<Path>>(file: &P) -> Result<DebugInfo, DebugInfoError> {
        let mut symbol_unit_mapping = HashMap::new();
        let file = std::fs::read(file)?;
        let object = object::File::parse(file.as_slice())?;

        // Load a section and return as `Cow<[u8]>`.
        let load_section = |id: gimli::SectionId| -> Result<DwarfReader, gimli::Error> {
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

        while let Ok(Some(header)) = iter.next() {
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
            };
        }

        Ok(DebugInfo {
            units,
            symbol_unit_mapping,
        })
    }

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

    pub fn size_from_kind(
        &self,
        kind: unit_info::DebugItemOffset,
    ) -> Option<unit_info::StructOffset> {
        self.symbol_unit_mapping
            .get(&kind)
            .map(|var| self.units[*var].size_from_kind(kind))
            .flatten()
    }

    pub fn variable_from_kind(
        &self,
        kind: unit_info::DebugItemOffset,
    ) -> Option<&unit_info::Variable> {
        self.symbol_unit_mapping
            .get(&kind)
            .map(|var| self.units[*var].variable_from_kind(kind))
            .flatten()
    }

    pub fn structure_from_kind(
        &self,
        kind: unit_info::DebugItemOffset,
    ) -> Option<&unit_info::Structure> {
        self.symbol_unit_mapping
            .get(&kind)
            .map(|var| self.units[*var].structure_from_kind(kind))
            .flatten()
    }

    pub fn enumeration_from_kind(
        &self,
        kind: unit_info::DebugItemOffset,
    ) -> Option<&unit_info::Enumeration> {
        self.symbol_unit_mapping
            .get(&kind)
            .map(|var| self.units[*var].enumeration_from_kind(kind))
            .flatten()
    }

    pub fn pointer_from_kind(
        &self,
        kind: unit_info::DebugItemOffset,
    ) -> Option<&unit_info::Pointer> {
        self.symbol_unit_mapping
            .get(&kind)
            .map(|var| self.units[*var].pointer_from_kind(kind))
            .flatten()
    }

    pub fn array_from_kind(&self, kind: unit_info::DebugItemOffset) -> Option<&unit_info::Array> {
        self.symbol_unit_mapping
            .get(&kind)
            .map(|var| self.units[*var].array_from_kind(kind))
            .flatten()
    }

    pub fn union_from_kind(&self, kind: unit_info::DebugItemOffset) -> Option<&unit_info::Union> {
        self.symbol_unit_mapping
            .get(&kind)
            .map(|var| self.units[*var].union_from_kind(kind))
            .flatten()
    }

    pub fn base_type_from_kind(
        &self,
        kind: unit_info::DebugItemOffset,
    ) -> Option<&unit_info::BaseType> {
        self.symbol_unit_mapping
            .get(&kind)
            .map(|var| self.units[*var].base_type_from_kind(kind))
            .flatten()
    }
}
