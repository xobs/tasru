use std::fmt::Debug;

use crate::{
    memory::Read,
    unit_info::{self, MemoryLocation, StructOffset},
    DebugInfo,
};

#[derive(Debug)]
pub enum DebugTypeError {
    /// Multiple items matched the given path
    MultipleMatches,
    MemberNotFound {
        owner: String,
        member: String,
    },
    GenericNotFound {
        owner: String,
    },
    StructureNotFound {
        owner: String,
    },
    BaseTypeNotFound {
        owner: String,
    },
    UnionNotFound {
        owner: String,
    },
    VariantNotFound {
        owner: String,
        variant: String,
    },
    EnumerationNotFound {
        owner: String,
    },
    ArrayNotFound(String),
    KindNotFound {
        owner: String,
        member: Option<String>,
    },
    KindIncorrect {
        owner: String,
        member: Option<String>,
        attempted: String,
        actual: String,
    },
    NotRustSice(String),
    ReadError,
    SizeError(u64),
    LocationMissing,
    VariableNotFound(String),
}

impl core::fmt::Display for DebugTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebugTypeError::StructureNotFound { owner } => {
                write!(f, "Structure for \"{}\" could not be found", owner)
            }
            DebugTypeError::EnumerationNotFound { owner } => {
                write!(f, "Enumeration for \"{}\" could not be found", owner)
            }
            DebugTypeError::VariantNotFound { owner, variant } => write!(
                f,
                "Variant \"{}\" could not be found in enum \"{}\"",
                variant, owner
            ),
            DebugTypeError::UnionNotFound { owner } => {
                write!(f, "Union \"{}\" could not be found", owner)
            }
            DebugTypeError::BaseTypeNotFound { owner } => {
                write!(f, "Base type \"{}\" could not be found", owner)
            }
            DebugTypeError::ArrayNotFound(s) => {
                write!(f, "Array could not be found for item \"{}\"", s)
            }
            DebugTypeError::MemberNotFound { owner, member } => {
                write!(f, "Member \"{}\" not found in struct \"{}\"", member, owner)
            }
            DebugTypeError::GenericNotFound { owner } => {
                write!(f, "Generic not found in struct \"{}\"", owner)
            }
            DebugTypeError::VariableNotFound(v) => {
                write!(f, "Variable \"{}\" could not be found", v)
            }
            DebugTypeError::SizeError(size) => write!(f, "Size \"{}\" is not valid", size),
            DebugTypeError::KindNotFound { owner, member } => {
                if let Some(member) = member {
                    write!(
                        f,
                        "Type for element \"{}\" in struct \"{}\" could not be found",
                        member, owner
                    )
                } else {
                    write!(
                        f,
                        "Type for anonymous member of struct \"{}\" could not be found",
                        owner
                    )
                }
            }
            DebugTypeError::KindIncorrect {
                owner,
                member,
                attempted,
                actual,
            } => {
                if let Some(member) = member {
                    write!(
                        f,
                        "Type for element \"{}\" in struct \"{}\" is \"{}\", not \"{}\"",
                        member, owner, actual, attempted
                    )
                } else {
                    write!(
                        f,
                        "Type for element \"{}\" \"{}\", not \"{}\"",
                        owner, actual, attempted
                    )
                }
            }
            DebugTypeError::NotRustSice(owner) => {
                write!(f, "Type \"{}\" is not a Rust slice", owner)
            }
            DebugTypeError::ReadError => {
                write!(f, "An error occurred when reading memory from the target")
            }
            DebugTypeError::LocationMissing => write!(f, "There was no location data available"),
            DebugTypeError::MultipleMatches => {
                write!(f, "Multiple items matched the specified string")
            }
        }
    }
}

impl core::error::Error for DebugTypeError {}

pub struct DebugArrayItem<'a> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    kind: unit_info::DebugItem,
    parent_name: String,
}

impl core::fmt::Debug for DebugArrayItem<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugArrayItem")
            .field("location", &self.location)
            .field("offset", &self.offset)
            .field("kind", &self.kind)
            .finish()
    }
}

/// An item stored within an array. The item may be a Structure, an Enumeration,
/// or a BaseType of u8.
impl<'a> DebugArrayItem<'a> {
    /// If the Array is an array of structs, return the underlying Structure object.
    pub fn structure(&self) -> Result<DebugStructure<'a>, DebugTypeError> {
        self.info
            .structure_from_item(self.kind)
            .map(|structure| DebugStructure {
                unit: self.unit,
                info: self.info,
                location: self.location,
                offset: self.offset,
                structure,
            })
            .ok_or(DebugTypeError::StructureNotFound {
                owner: self.parent_name.clone(),
            })
    }

    /// If the Array is an array of enums, return the underlying Enumeration object.
    pub fn enumeration(&self) -> Result<DebugEnumeration<'a>, DebugTypeError> {
        self.info
            .enumeration_from_item(self.kind)
            .map(|enumeration| DebugEnumeration {
                unit: self.unit,
                info: self.info,
                location: self.location,
                offset: self.offset,
                enumeration,
            })
            .ok_or_else(|| DebugTypeError::EnumerationNotFound {
                owner: self.parent_name.clone(),
            })
    }

    /// Treat the Array as a `u8`. This can be useful for reading strings, which are
    /// generally stored as arrays of u8 values.
    pub fn u8<S: Read + ?Sized>(&self, memory_source: &mut S) -> Option<u8> {
        if let Some(location) = self.location {
            if let Some(base_type) = self.info.base_type_from_item(self.kind) {
                if base_type.size() == 1 {
                    return memory_source.read_u8(location.0).ok();
                }
            }
        }
        None
    }
}

/// An iterator over array items.
pub struct DebugArrayIterator<'a> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    array: &'a unit_info::Array,
    index: usize,
    count: usize,
    element_size: StructOffset,
    parent_name: String,
}

impl<'a> Iterator for DebugArrayIterator<'a> {
    type Item = DebugArrayItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }
        let location = self
            .location
            .map(|loc| loc + self.element_size * StructOffset::new(self.index as u64));
        self.index += 1;
        Some(DebugArrayItem {
            unit: self.unit,
            info: self.info,
            location,
            offset: self.offset,
            kind: self.array.kind(),
            parent_name: self.parent_name.clone(),
        })
    }
}

/// An array of values in memory. The size of the array is taken from the Dwarf data and
/// is fixed at compile time.
pub struct DebugArray<'a> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    array: &'a unit_info::Array,
    parent_name: String,
}

impl<'a> DebugArray<'a> {
    pub fn structure(&self) -> Option<DebugStructure<'a>> {
        self.info
            .structure_from_item(self.array.kind())
            .map(|structure| DebugStructure {
                unit: self.unit,
                info: self.info,
                location: self.location,
                offset: self.offset,
                structure,
            })
    }

    pub fn enumeration(&self) -> Option<DebugEnumeration<'a>> {
        self.info
            .enumeration_from_item(self.array.kind())
            .map(|enumeration| DebugEnumeration {
                unit: self.unit,
                info: self.info,
                location: self.location,
                offset: self.offset,
                enumeration,
            })
    }

    pub fn iter(&self) -> Result<DebugArrayIterator<'a>, DebugTypeError> {
        let element_size = self.info.size_from_item(self.array.kind()).ok_or_else(|| {
            DebugTypeError::KindNotFound {
                owner: self.parent_name.clone(),
                member: None,
            }
        })?;
        let count = self.count();
        Ok(DebugArrayIterator {
            unit: self.unit,
            info: self.info,
            location: self.location,
            offset: self.offset,
            array: self.array,
            index: 0,
            count,
            element_size,
            parent_name: self.parent_name.clone(),
        })
    }

    pub fn reset_offset(&mut self) -> &Self {
        self.offset = unit_info::StructOffset::new(0);
        self
    }
}

impl core::ops::Deref for DebugArray<'_> {
    type Target = unit_info::Array;

    fn deref(&self) -> &Self::Target {
        self.array
    }
}

impl core::fmt::Debug for DebugArray<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugArray")
            .field("location", &self.location)
            .field("offset", &self.offset)
            .field("array", &self.array)
            .finish()
    }
}

pub struct DebugBaseType<'a> {
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    base_type: &'a unit_info::BaseType,
}

impl DebugBaseType<'_> {
    pub fn name(&self) -> &str {
        self.base_type.name()
    }

    pub fn size(&self) -> u64 {
        self.base_type.size()
    }

    pub fn as_u8<S: Read + ?Sized>(&self, memory_source: &mut S) -> Option<u8> {
        let address = self.location?.0;
        Some(match self.size() {
            1 => memory_source.read_u8(address).ok()?,
            _ => return None,
        })
    }

    pub fn as_u16<S: Read + ?Sized>(&self, memory_source: &mut S) -> Option<u16> {
        let address = self.location?.0;
        Some(match self.size() {
            1 => memory_source.read_u8(address).ok()?.into(),
            2 => memory_source.read_u16(address).ok()?,
            _ => return None,
        })
    }

    pub fn as_u32<S: Read + ?Sized>(&self, memory_source: &mut S) -> Option<u32> {
        let address = self.location?.0;
        Some(match self.size() {
            1 => memory_source.read_u8(address).ok()?.into(),
            2 => memory_source.read_u16(address).ok()?.into(),
            4 => memory_source.read_u32(address).ok()?,
            _ => return None,
        })
    }

    pub fn as_u64<S: Read + ?Sized>(&self, memory_source: &mut S) -> Option<u64> {
        let address = self.location?.0;
        Some(match self.size() {
            1 => memory_source.read_u8(address).ok()?.into(),
            2 => memory_source.read_u16(address).ok()?.into(),
            4 => memory_source.read_u32(address).ok()?.into(),
            8 => memory_source.read_u64(address).ok()?,
            _ => return None,
        })
    }
}

impl core::fmt::Debug for DebugBaseType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugBaseType")
            .field("location", &self.location)
            .field("offset", &self.offset)
            .field("base_type", &self.base_type)
            .finish()
    }
}

pub struct DebugStructureMember<'a> {
    parent_name: String,
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    structure_member: &'a unit_info::StructureMember,
}

impl<'a> DebugStructureMember<'a> {
    fn find_alternatives(&self, attempted: &str) -> DebugTypeError {
        let member = self.structure_member.name().map(|s| s.to_owned());
        let attempted = attempted.to_owned();
        let kind_index = self.structure_member.kind();
        if self.info.structure_from_item(kind_index).is_some() {
            DebugTypeError::KindIncorrect {
                owner: self.parent_name.clone(),
                member,
                attempted,
                actual: "structure".to_owned(),
            }
        } else if self.info.enumeration_from_item(kind_index).is_some() {
            DebugTypeError::KindIncorrect {
                owner: self.parent_name.clone(),
                member,
                attempted,
                actual: "enumeration".to_owned(),
            }
        } else if self.info.pointer_from_item(kind_index).is_some() {
            DebugTypeError::KindIncorrect {
                owner: self.parent_name.clone(),
                member,
                attempted,
                actual: "pointer".to_owned(),
            }
        } else if self.info.array_from_item(kind_index).is_some() {
            DebugTypeError::KindIncorrect {
                owner: self.parent_name.clone(),
                member,
                attempted,
                actual: "array".to_owned(),
            }
        } else if self.info.union_from_item(kind_index).is_some() {
            DebugTypeError::KindIncorrect {
                owner: self.parent_name.clone(),
                member,
                attempted,
                actual: "union".to_owned(),
            }
        } else if self.info.base_type_from_item(kind_index).is_some() {
            DebugTypeError::KindIncorrect {
                owner: self.parent_name.clone(),
                member,
                attempted,
                actual: "base type".to_owned(),
            }
        } else {
            DebugTypeError::KindNotFound {
                owner: self.parent_name.clone(),
                member,
            }
        }
    }

    pub fn structure(&self) -> Result<DebugStructure<'a>, DebugTypeError> {
        self.info
            .structure_from_item(self.structure_member.kind())
            .map(|structure| DebugStructure {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + self.structure_member.offset()),
                offset: self.offset + self.structure_member.offset(),
                structure,
            })
            .ok_or_else(|| self.find_alternatives("structure"))
    }

    pub fn enumeration(&self) -> Result<DebugEnumeration<'a>, DebugTypeError> {
        self.info
            .enumeration_from_item(self.structure_member.kind())
            .map(|enumeration| DebugEnumeration {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + self.structure_member.offset()),
                offset: self.offset + self.structure_member.offset(),
                enumeration,
            })
            .ok_or_else(|| self.find_alternatives("enumeration"))
    }

    pub fn pointer(&self) -> Result<DebugPointer<'a>, DebugTypeError> {
        self.info
            .pointer_from_item(self.structure_member.kind())
            .map(|pointer| DebugPointer {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + self.structure_member.offset()),
                offset: self.offset + self.structure_member.offset(),
                pointer,
                parent_name: self.parent_name.clone(),
            })
            .ok_or_else(|| self.find_alternatives("pointer"))
    }

    pub fn array(&self) -> Result<DebugArray<'a>, DebugTypeError> {
        self.info
            .array_from_item(self.structure_member.kind())
            .map(|array| DebugArray {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + self.structure_member.offset()),
                offset: self.offset + self.structure_member.offset(),
                array,
                parent_name: self.parent_name.clone(),
            })
            .ok_or_else(|| self.find_alternatives("array"))
    }

    pub fn union(&self) -> Result<DebugUnion<'a>, DebugTypeError> {
        self.info
            .union_from_item(self.structure_member.kind())
            .map(|union| DebugUnion {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + self.structure_member.offset()),
                offset: self.offset + self.structure_member.offset(),
                union,
            })
            .ok_or_else(|| self.find_alternatives("union"))
    }

    pub fn base_type(&self) -> Result<DebugBaseType<'a>, DebugTypeError> {
        self.info
            .base_type_from_item(self.structure_member.kind())
            .map(|base_type| DebugBaseType {
                location: self.location.map(|l| l + self.structure_member.offset()),
                offset: self.offset + self.structure_member.offset(),
                base_type,
            })
            .ok_or_else(|| self.find_alternatives("base type"))
    }

    pub fn reset_offset(&mut self) -> &Self {
        self.offset = unit_info::StructOffset::new(0);
        self
    }

    pub fn location(&self) -> Result<u64, DebugTypeError> {
        self.location
            .ok_or(DebugTypeError::LocationMissing)
            .map(|location| location.0)
    }
}

impl core::ops::Deref for DebugStructureMember<'_> {
    type Target = unit_info::StructureMember;

    fn deref(&self) -> &Self::Target {
        self.structure_member
    }
}

impl core::fmt::Debug for DebugStructureMember<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugStructureMember")
            .field("structure_member", &self.structure_member)
            .finish()
    }
}

pub struct DebugUnion<'a> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    union: &'a unit_info::Union,
}

impl<'a> DebugUnion<'a> {
    pub(crate) fn new(
        unit: &'a unit_info::UnitInfo,
        info: &'a DebugInfo,
        union: &'a unit_info::Union,
        location: unit_info::MemoryLocation,
    ) -> Self {
        Self {
            unit,
            info,
            location: Some(location),
            offset: unit_info::StructOffset::new(0),
            union,
        }
    }

    pub fn member_named(&self, name: &str) -> Result<DebugStructureMember<'a>, DebugTypeError> {
        self.union
            .member_named(name)
            .map(|structure_member| DebugStructureMember {
                unit: self.unit,
                info: self.info,
                location: self.location,
                offset: self.offset + structure_member.offset(),
                parent_name: self.union.name().into(),
                structure_member,
            })
            .ok_or_else(|| DebugTypeError::UnionNotFound {
                owner: self.union.name().to_string(),
            })
    }

    pub fn location(&self) -> Option<unit_info::MemoryLocation> {
        self.location
    }
}

impl core::fmt::Debug for DebugUnion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugUnion")
            .field("union", &self.union)
            .finish()
    }
}
pub struct DebugSliceBaseTypeIter<'a> {
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    length: u64,
    current: u64,
    size: unit_info::StructOffset,
    base_type: &'a unit_info::BaseType,
}

impl DebugSliceBaseTypeIter<'_> {
    pub fn len(&self) -> usize {
        self.length as usize
    }
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}

impl<'a> Iterator for DebugSliceBaseTypeIter<'a> {
    type Item = DebugBaseType<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.length {
            return None;
        }
        let current = unit_info::StructOffset::new(self.current);
        let new = DebugBaseType {
            location: self.location.map(|l| l + self.size * current),
            offset: self.offset + self.size * current,
            base_type: self.base_type,
        };
        self.current += 1;
        Some(new)
    }
}

pub struct DebugSliceStructureIter<'a> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    length: u64,
    current: u64,
    size: unit_info::StructOffset,
    structure: &'a unit_info::Structure,
}

impl DebugSliceStructureIter<'_> {
    pub fn len(&self) -> usize {
        self.length as usize
    }
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}

impl<'a> Iterator for DebugSliceStructureIter<'a> {
    type Item = DebugStructure<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.length {
            return None;
        }
        let current = unit_info::StructOffset::new(self.current);
        let new = DebugStructure {
            unit: self.unit,
            info: self.info,
            location: self.location.map(|l| l + self.size * current),
            offset: self.offset + self.size * current,
            structure: self.structure,
        };
        self.current += 1;
        Some(new)
    }
}

/// Wrap a Structure to include the unit that it came from
pub struct DebugSlice<'a> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    length: u64,
    data_ptr: &'a unit_info::Pointer,
    parent_name: String,
}

impl<'a> DebugSlice<'a> {
    pub fn base_type_iter(&self) -> Result<DebugSliceBaseTypeIter<'a>, DebugTypeError> {
        let Some(base_type) = self.info.base_type_from_item(self.data_ptr.kind()) else {
            return Err(DebugTypeError::BaseTypeNotFound {
                owner: self.parent_name.clone(),
            });
        };
        let Some(element_size) = self.info.size_from_item(self.data_ptr.kind()) else {
            return Err(DebugTypeError::KindNotFound {
                owner: "<todo>".into(),
                member: None,
            });
        };
        Ok(DebugSliceBaseTypeIter {
            location: self.location,
            offset: self.offset,
            length: self.length,
            current: 0,
            size: element_size,
            base_type,
        })
    }

    pub fn structure_iter(&self) -> Result<DebugSliceStructureIter<'a>, DebugTypeError> {
        let structure = self
            .info
            .structure_from_item(self.data_ptr.kind())
            .ok_or_else(|| DebugTypeError::StructureNotFound {
                owner: self.parent_name.clone(),
            })?;
        let element_size = self
            .info
            .size_from_item(self.data_ptr.kind())
            .ok_or_else(|| DebugTypeError::KindNotFound {
                owner: self.parent_name.clone(),
                member: None,
            })?;

        Ok(DebugSliceStructureIter {
            unit: self.unit,
            info: self.info,
            location: self.location,
            offset: self.offset,
            length: self.length,
            current: 0,
            size: element_size,
            structure,
        })
    }
}

/// Wrap a Structure to include the unit that it came from
pub struct DebugStructure<'a> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    structure: &'a unit_info::Structure,
}

impl<'a> DebugStructure<'a> {
    pub(crate) fn new(
        unit: &'a unit_info::UnitInfo,
        info: &'a DebugInfo,
        structure: &'a unit_info::Structure,
        location: unit_info::MemoryLocation,
    ) -> Self {
        DebugStructure {
            unit,
            info,
            location: Some(location),
            offset: unit_info::StructOffset::new(0),
            structure,
        }
    }

    pub fn member_named(&self, name: &str) -> Result<DebugStructureMember<'a>, DebugTypeError> {
        self.structure
            .member_named(name)
            .map(|structure_member| DebugStructureMember {
                unit: self.unit,
                info: self.info,
                location: self.location,
                offset: self.offset + structure_member.offset(),
                parent_name: self.structure.name().into(),
                structure_member,
            })
            .ok_or_else(|| DebugTypeError::MemberNotFound {
                owner: self.structure.name().into(),
                member: name.into(),
            })
    }

    /// Special case for Rust slices, which always have two members:
    /// a "data_ptr" and a "length".
    pub fn as_slice<S: Read + ?Sized>(
        &self,
        memory_source: &mut S,
    ) -> Result<DebugSlice<'a>, DebugTypeError> {
        if self.structure.members().len() != 2 {
            return Err(DebugTypeError::NotRustSice(self.structure.name().into()));
        }
        let length = self
            .member_named("length")?
            .base_type()?
            .as_u64(memory_source)
            .ok_or(DebugTypeError::ReadError)?;
        let data_ptr = self
            .member_named("data_ptr")?
            .pointer()?
            .follow_unless_null(memory_source)?;
        Ok(DebugSlice {
            unit: self.unit,
            info: self.info,
            location: data_ptr.location,
            offset: self.offset,
            length,
            data_ptr: data_ptr.pointer,
            parent_name: self.structure.name().to_string(),
        })
    }

    pub fn location(&self) -> Option<unit_info::MemoryLocation> {
        self.location
    }
}

impl core::fmt::Debug for DebugStructure<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugStructure")
            .field("structure", &self.structure)
            .finish()
    }
}

impl core::ops::Deref for DebugStructure<'_> {
    type Target = unit_info::Structure;

    fn deref(&self) -> &Self::Target {
        self.structure
    }
}

/// Wrap a Pointer to include the unit that it came from
pub struct DebugPointer<'a> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    pointer: &'a unit_info::Pointer,
    parent_name: String,
}

impl<'a> DebugPointer<'a> {
    pub fn structure(&self) -> Result<DebugStructure<'a>, DebugTypeError> {
        self.info
            .structure_from_item(self.pointer.kind())
            .map(|structure| DebugStructure {
                unit: self.unit,
                structure,
                location: self.location,
                offset: self.offset,
                info: self.info,
            })
            .ok_or_else(|| DebugTypeError::StructureNotFound {
                owner: self.parent_name.clone(),
            })
    }

    pub fn follow_unless_null<S: Read + ?Sized>(
        self,
        memory_source: &mut S,
    ) -> Result<Self, DebugTypeError> {
        let new = self.follow(memory_source)?;
        let location = &new.location.ok_or(DebugTypeError::ReadError)?;
        if *location == MemoryLocation(0) {
            Err(DebugTypeError::ReadError)
        } else {
            Ok(new)
        }
    }

    /// Attempt to follow a pointer. If the pointer is null, return `Ok(None)`.
    pub fn try_follow<S: Read + ?Sized>(
        self,
        memory_source: &mut S,
    ) -> Result<Option<Self>, DebugTypeError> {
        let new = self.follow(memory_source)?;
        let location = &new.location.ok_or(DebugTypeError::ReadError)?;
        if *location == MemoryLocation(0) {
            Ok(None)
        } else {
            Ok(Some(new))
        }
    }

    pub fn follow<S: Read + ?Sized>(
        mut self,
        memory_source: &mut S,
    ) -> Result<Self, DebugTypeError> {
        let location = self.location.ok_or(DebugTypeError::LocationMissing)?.0;
        let target = memory_source
            .read_u32(location)
            .map_err(|_| DebugTypeError::ReadError)?;
        self.location = Some(MemoryLocation(target.into()));
        self.offset = StructOffset::new(0);
        Ok(self)
    }

    /// Read a u8 from the specified offset
    pub fn read_u8<S: Read + ?Sized>(&self, offset: u64, memory_source: &mut S) -> Option<u8> {
        let location = self.location?.0 + offset;
        memory_source.read_u8(location).ok()
    }

    pub fn location(&self) -> Result<u64, DebugTypeError> {
        self.location
            .ok_or(DebugTypeError::LocationMissing)
            .map(|location| location.0)
    }
}

impl core::fmt::Debug for DebugPointer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugPointer")
            .field("pointer", &self.pointer)
            .field("location", &self.location)
            .finish()
    }
}

impl core::ops::Deref for DebugPointer<'_> {
    type Target = unit_info::Pointer;

    fn deref(&self) -> &Self::Target {
        self.pointer
    }
}

/// Wrap an Enumeration to include the unit that it came from
pub struct DebugEnumerationVariant<'a> {
    parent_name: String,
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    variant: &'a unit_info::EnumerationVariant,
}

impl<'a> DebugEnumerationVariant<'a> {
    pub fn structure(&self) -> Result<DebugStructure<'a>, DebugTypeError> {
        self.info
            .structure_from_item(self.variant.kind())
            .map(|structure| DebugStructure {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + self.variant.offset()),
                offset: self.offset + self.variant.offset(),
                structure,
            })
            .ok_or_else(|| DebugTypeError::StructureNotFound {
                owner: self.parent_name.clone(),
            })
    }
}

impl core::fmt::Debug for DebugEnumerationVariant<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugEnumerationVariant")
            .field("variant", &self.variant)
            .finish()
    }
}

impl core::ops::Deref for DebugEnumerationVariant<'_> {
    type Target = unit_info::EnumerationVariant;

    fn deref(&self) -> &Self::Target {
        self.variant
    }
}

/// Wrap an Enumeration to include the unit that it came from
pub struct DebugEnumeration<'a> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    enumeration: &'a unit_info::Enumeration,
}

impl<'a> DebugEnumeration<'a> {
    pub(crate) fn new(
        unit: &'a unit_info::UnitInfo,
        info: &'a DebugInfo,
        enumeration: &'a unit_info::Enumeration,
        location: unit_info::MemoryLocation,
    ) -> Self {
        Self {
            unit,
            info,
            location: Some(location),
            offset: unit_info::StructOffset::new(0),
            enumeration,
        }
    }

    pub fn discriminant_size(&self) -> Result<u64, DebugTypeError> {
        let discriminant = self
            .info
            .base_type_from_item(self.enumeration.discriminant_kind())
            .ok_or_else(|| DebugTypeError::BaseTypeNotFound {
                owner: self.enumeration.name().to_owned(),
            })?;
        Ok(discriminant.size())
    }

    /// Look through the list of variants and get the variant with the
    /// discriminant that matches the given index.
    pub fn variant_with_discriminant(
        &self,
        discriminant: usize,
    ) -> Result<DebugEnumerationVariant<'a>, DebugTypeError> {
        self.enumeration
            .variant_with_discriminant(discriminant)
            .map(|variant| DebugEnumerationVariant {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + variant.offset()),
                offset: self.offset + variant.offset(),
                variant,
                parent_name: self.enumeration.name().to_owned(),
            })
            .ok_or_else(|| DebugTypeError::VariantNotFound {
                owner: self.enumeration.name().to_owned(),
                variant: format!("{}", discriminant),
            })
    }

    /// Return the enum variant with the given name. If no variant could be found, return `None`.
    pub fn variant_named(&self, name: &str) -> Result<DebugEnumerationVariant<'a>, DebugTypeError> {
        self.enumeration
            .variant_named(name)
            .map(|variant| DebugEnumerationVariant {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + variant.offset()),
                offset: self.offset + variant.offset(),
                variant,
                parent_name: self.enumeration.name().to_owned(),
            })
            .ok_or_else(|| DebugTypeError::VariantNotFound {
                owner: self.enumeration.name().to_owned(),
                variant: name.to_owned(),
            })
    }

    pub fn location(&self) -> Result<u64, DebugTypeError> {
        self.location
            .ok_or(DebugTypeError::LocationMissing)
            .map(|location| location.0)
    }

    pub fn variants(&self) -> Result<Vec<DebugEnumerationVariant<'a>>, DebugTypeError> {
        let mut variants = vec![];
        for variant in self.enumeration.variants() {
            variants.push(DebugEnumerationVariant {
                parent_name: self.name().to_owned(),
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + variant.offset()),
                offset: self.offset + variant.offset(),
                variant,
            })
        }
        Ok(variants)
    }

    /// Returns the currently-selected variant, if one is available.
    pub fn variant<S: Read + ?Sized>(
        &self,
        memory_source: &mut S,
    ) -> Result<DebugEnumerationVariant<'a>, DebugTypeError> {
        let address = self.location.ok_or(DebugTypeError::LocationMissing)?.0;
        let discriminant_size = self
            .info
            .size_from_item(self.discriminant_kind())
            .ok_or_else(|| DebugTypeError::KindNotFound {
                owner: self.enumeration.name().to_owned(),
                member: None,
            })?;
        let discriminant: u64 = match discriminant_size.0 {
            1 => memory_source
                .read_u8(address)
                .map_err(|_| DebugTypeError::ReadError)?
                .into(),
            2 => memory_source
                .read_u16(address)
                .map_err(|_| DebugTypeError::ReadError)?
                .into(),
            4 => memory_source
                .read_u32(address)
                .map_err(|_| DebugTypeError::ReadError)?
                .into(),
            8 => memory_source
                .read_u64(address)
                .map_err(|_| DebugTypeError::ReadError)?,
            size => return Err(DebugTypeError::SizeError(size)),
        };
        self.variant_with_discriminant(discriminant as usize)
    }
}

impl core::fmt::Debug for DebugEnumeration<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugEnumeration")
            .field("enumeration", &self.enumeration)
            .finish()
    }
}

impl core::ops::Deref for DebugEnumeration<'_> {
    type Target = unit_info::Enumeration;

    fn deref(&self) -> &Self::Target {
        self.enumeration
    }
}

/// Wrap a Variable to include the unit that it came from
pub struct DebugVariable<'a> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo,
    variable: &'a unit_info::Variable,
}

impl<'a> DebugVariable<'a> {
    pub fn new(
        unit: &'a unit_info::UnitInfo,
        info: &'a DebugInfo,
        variable: &'a unit_info::Variable,
    ) -> Self {
        DebugVariable {
            unit,
            info,
            variable,
        }
    }

    pub fn structure(&self) -> Result<DebugStructure<'a>, DebugTypeError> {
        self.info
            .structure_from_item(self.variable.kind())
            .map(|structure| DebugStructure {
                unit: self.unit,
                info: self.info,
                location: Some(self.variable.location()),
                offset: unit_info::StructOffset::new(0),
                structure,
            })
            .ok_or_else(|| DebugTypeError::StructureNotFound {
                owner: self.variable.name().to_string(),
            })
    }

    pub fn enumeration(&self) -> Result<DebugEnumeration<'a>, DebugTypeError> {
        self.info
            .enumeration_from_item(self.variable.kind())
            .map(|enumeration| DebugEnumeration {
                unit: self.unit,
                info: self.info,
                location: Some(self.variable.location()),
                offset: unit_info::StructOffset::new(0),
                enumeration,
            })
            .ok_or_else(|| DebugTypeError::EnumerationNotFound {
                owner: self.variable.name().to_string(),
            })
    }

    pub fn array(&self) -> Result<DebugArray<'a>, DebugTypeError> {
        self.info
            .array_from_item(self.variable.kind())
            .map(|array| DebugArray {
                unit: self.unit,
                info: self.info,
                location: Some(self.variable.location()),
                offset: unit_info::StructOffset::new(0),
                array,
                parent_name: self.variable.name().to_string(),
            })
            .ok_or(DebugTypeError::ArrayNotFound(self.variable.name().into()))
    }
}

impl core::ops::Deref for DebugVariable<'_> {
    type Target = unit_info::Variable;

    fn deref(&self) -> &Self::Target {
        self.variable
    }
}

impl core::fmt::Debug for DebugVariable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugVariable")
            // .field("unit", &self.unit)
            .field("variable", &self.variable)
            .finish()
    }
}
