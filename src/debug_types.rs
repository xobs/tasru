use crate::{
    memory_source::MemorySource,
    unit_info::{self, MemoryLocation, StructOffset},
    DebugInfo,
};

pub struct DebugArrayItem<'a, S: MemorySource> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo<'a, S>,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    kind: unit_info::DebugItemOffset,
}

impl<'a, S: MemorySource> core::fmt::Debug for DebugArrayItem<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugArrayItem")
            .field("location", &self.location)
            .field("offset", &self.offset)
            .field("kind", &self.kind)
            .finish()
    }
}

impl<'a, S: MemorySource> DebugArrayItem<'a, S> {
    pub fn structure(&self) -> Option<DebugStructure<'a, S>> {
        self.info
            .structure_from_kind(self.kind)
            .map(|structure| DebugStructure {
                unit: self.unit,
                info: self.info,
                location: self.location,
                offset: self.offset,
                structure,
            })
    }
    pub fn enumeration(&self) -> Option<DebugEnumeration<'a, S>> {
        self.info
            .enumeration_from_kind(self.kind)
            .map(|enumeration| DebugEnumeration {
                unit: self.unit,
                info: self.info,
                location: self.location,
                offset: self.offset,
                enumeration,
            })
    }
    pub fn u8(&self) -> Option<u8> {
        if let Some(location) = self.location {
            if let Some(base_type) = self.info.base_type_from_kind(self.kind) {
                if base_type.size() == 1 {
                    return self.info.memory_source.read_u8(location.0).ok();
                }
            }
        }
        None
    }
}

pub struct DebugArrayIterator<'a, S: MemorySource> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo<'a, S>,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    array: &'a unit_info::Array,
    index: usize,
    count: usize,
    element_size: StructOffset,
}

impl<'a, S: MemorySource> Iterator for DebugArrayIterator<'a, S> {
    type Item = DebugArrayItem<'a, S>;

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
        })
    }
}

pub struct DebugArray<'a, S: MemorySource> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo<'a, S>,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    array: &'a unit_info::Array,
}

impl<'a, S: MemorySource> DebugArray<'a, S> {
    pub fn structure(&self) -> Option<DebugStructure<'a, S>> {
        if let Some(structure) = self.info.structure_from_kind(self.array.kind()) {
            Some(DebugStructure {
                unit: self.unit,
                info: self.info,
                location: self.location,
                offset: self.offset,
                structure,
            })
        } else {
            None
        }
    }

    pub fn enumeration(&self) -> Option<DebugEnumeration<'a, S>> {
        if let Some(enumeration) = self.info.enumeration_from_kind(self.array.kind()) {
            Some(DebugEnumeration {
                unit: self.unit,
                info: self.info,
                location: self.location,
                offset: self.offset,
                enumeration,
            })
        } else {
            None
        }
    }

    pub fn iter(&self) -> Option<DebugArrayIterator<'a, S>> {
        let element_size = self.info.size_from_kind(self.array.kind())?;
        // let name = self.unit.name_from_kind(self.array.kind())?;
        // println!("Item is {} bytes long and is called {}", element_size, name);
        // println!("WARNING! Setting count to 2");
        // let count = 2; // Should be self.count()
        let count = self.count();
        Some(DebugArrayIterator {
            unit: self.unit,
            info: self.info,
            location: self.location,
            offset: self.offset,
            array: self.array,
            index: 0,
            count,
            element_size,
        })
    }

    pub fn reset_offset(&mut self) -> &Self {
        self.offset = unit_info::StructOffset::new(0);
        self
    }
}

impl<'a, S: MemorySource> core::ops::Deref for DebugArray<'a, S> {
    type Target = unit_info::Array;

    fn deref(&self) -> &Self::Target {
        self.array
    }
}

impl<'a, S: MemorySource> core::fmt::Debug for DebugArray<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugArray")
            .field("location", &self.location)
            .field("offset", &self.offset)
            .field("array", &self.array)
            .finish()
    }
}

pub struct DebugBaseType<'a, S: MemorySource> {
    // unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo<'a, S>,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    base_type: &'a unit_info::BaseType,
}

impl<'a, S: MemorySource> DebugBaseType<'a, S> {
    pub fn name(&self) -> &str {
        self.base_type.name()
    }

    pub fn size(&self) -> u64 {
        self.base_type.size()
    }

    pub fn as_u8(&self) -> Option<u8> {
        let address = self.location?.0;
        Some(match self.size() {
            1 => self.info.memory_source.read_u8(address).ok()?,
            _ => return None,
        })
    }

    pub fn as_u16(&self) -> Option<u16> {
        let address = self.location?.0;
        Some(match self.size() {
            1 => self.info.memory_source.read_u8(address).ok()?.into(),
            2 => self.info.memory_source.read_u16(address).ok()?.into(),
            _ => return None,
        })
    }

    pub fn as_u32(&self) -> Option<u32> {
        let address = self.location?.0;
        Some(match self.size() {
            1 => self.info.memory_source.read_u8(address).ok()?.into(),
            2 => self.info.memory_source.read_u16(address).ok()?.into(),
            4 => self.info.memory_source.read_u32(address).ok()?.into(),
            _ => return None,
        })
    }

    pub fn as_u64(&self) -> Option<u64> {
        let address = self.location?.0;
        Some(match self.size() {
            1 => self.info.memory_source.read_u8(address).ok()?.into(),
            2 => self.info.memory_source.read_u16(address).ok()?.into(),
            4 => self.info.memory_source.read_u32(address).ok()?.into(),
            8 => self.info.memory_source.read_u64(address).ok()?,
            _ => return None,
        })
    }
}

impl<'a, S: MemorySource> core::fmt::Debug for DebugBaseType<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugBaseType")
            .field("location", &self.location)
            .field("offset", &self.offset)
            .field("base_type", &self.base_type)
            .finish()
    }
}

pub struct DebugStructureMember<'a, S: MemorySource> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo<'a, S>,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    structure_member: &'a unit_info::StructureMember,
}

impl<'a, S: MemorySource> DebugStructureMember<'a, S> {
    fn find_alternatives<T>(&self, existing: &str) -> Option<T> {
        let kind = self.structure_member.kind();
        if self.info.structure_from_kind(kind).is_some() {
            println!("Warning: item is a structure (not {})", existing);
        } else if self.info.enumeration_from_kind(kind).is_some() {
            println!("Warning: item is an enumeration (not {})", existing);
        } else if self.info.pointer_from_kind(kind).is_some() {
            println!("Warning: item is a pointer (not {})", existing);
        } else if self.info.array_from_kind(kind).is_some() {
            println!("Warning: item is an array (not {})", existing);
        } else if self.info.union_from_kind(kind).is_some() {
            println!("Warning: item is a union (not {})", existing);
        } else if self.info.base_type_from_kind(kind).is_some() {
            println!("Warning: item is a base type (not {})", existing);
        }
        None
    }
    pub fn structure(&self) -> Option<DebugStructure<'a, S>> {
        self.info
            .structure_from_kind(self.structure_member.kind())
            .map(|structure| DebugStructure {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + self.structure_member.offset()),
                offset: self.offset + self.structure_member.offset(),
                structure,
            })
            .or_else(|| self.find_alternatives("structure"))
    }

    pub fn enumeration(&self) -> Option<DebugEnumeration<'a, S>> {
        self.info
            .enumeration_from_kind(self.structure_member.kind())
            .map(|enumeration| DebugEnumeration {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + self.structure_member.offset()),
                offset: self.offset + self.structure_member.offset(),
                enumeration,
            })
            .or_else(|| self.find_alternatives("enumeration"))
    }

    pub fn pointer(&self) -> Option<DebugPointer<'a, S>> {
        self.info
            .pointer_from_kind(self.structure_member.kind())
            .map(|pointer| DebugPointer {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + self.structure_member.offset()),
                offset: self.offset + self.structure_member.offset(),
                pointer,
            })
            .or_else(|| self.find_alternatives("pointer"))
    }

    pub fn array(&self) -> Option<DebugArray<'a, S>> {
        self.info
            .array_from_kind(self.structure_member.kind())
            .map(|array| DebugArray {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + self.structure_member.offset()),
                offset: self.offset + self.structure_member.offset(),
                array,
            })
            .or_else(|| self.find_alternatives("array"))
    }

    pub fn union(&self) -> Option<DebugUnion<'a, S>> {
        self.info
            .union_from_kind(self.structure_member.kind())
            .map(|union| DebugUnion {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + self.structure_member.offset()),
                offset: self.offset + self.structure_member.offset(),
                union,
            })
            .or_else(|| self.find_alternatives("union"))
    }

    pub fn base_type(&self) -> Option<DebugBaseType<'a, S>> {
        self.info
            .base_type_from_kind(self.structure_member.kind())
            .map(|base_type| DebugBaseType {
                // unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + self.structure_member.offset()),
                offset: self.offset + self.structure_member.offset(),
                base_type,
            })
            .or_else(|| self.find_alternatives("base type"))
    }

    pub fn reset_offset(&mut self) -> &Self {
        self.offset = unit_info::StructOffset::new(0);
        self
    }
}

impl<'a, S: MemorySource> core::ops::Deref for DebugStructureMember<'a, S> {
    type Target = unit_info::StructureMember;

    fn deref(&self) -> &Self::Target {
        self.structure_member
    }
}

impl<'a, S: MemorySource> core::fmt::Debug for DebugStructureMember<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugStructureMember")
            .field("structure_member", &self.structure_member)
            .finish()
    }
}

pub struct DebugUnion<'a, S: MemorySource> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo<'a, S>,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    union: &'a unit_info::Union,
}

impl<'a, S: MemorySource> DebugUnion<'a, S> {
    pub fn member_named(&self, name: &str) -> Option<DebugStructureMember<'a, S>> {
        self.union
            .member_named(name)
            .map(|structure_member| DebugStructureMember {
                unit: self.unit,
                info: self.info,
                location: self.location,
                offset: self.offset + structure_member.offset(),
                structure_member,
            })
    }

    pub fn location(&self) -> Option<unit_info::MemoryLocation> {
        self.location
    }
}

impl<'a, S: MemorySource> core::fmt::Debug for DebugUnion<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugUnion")
            .field("union", &self.union)
            .finish()
    }
}
pub struct DebugSliceBaseTypeIter<'a, S: MemorySource> {
    // unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo<'a, S>,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    length: u64,
    current: u64,
    size: unit_info::StructOffset,
    base_type: &'a unit_info::BaseType,
}

impl<'a, S: MemorySource> DebugSliceBaseTypeIter<'a, S> {
    pub fn len(&self) -> usize {
        self.length as usize
    }
}

impl<'a, S: MemorySource> Iterator for DebugSliceBaseTypeIter<'a, S> {
    type Item = DebugBaseType<'a, S>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.length {
            return None;
        }
        let current = unit_info::StructOffset::new(self.current);
        let new = DebugBaseType {
            // unit: self.unit,
            info: self.info,
            location: self.location.map(|l| l + self.size * current),
            offset: self.offset + self.size * current,
            base_type: self.base_type,
        };
        self.current += 1;
        Some(new)
    }
}

pub struct DebugSliceStructureIter<'a, S: MemorySource> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo<'a, S>,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    length: u64,
    current: u64,
    size: unit_info::StructOffset,
    structure: &'a unit_info::Structure,
}

impl<'a, S: MemorySource> DebugSliceStructureIter<'a, S> {
    pub fn len(&self) -> usize {
        self.length as usize
    }
}

impl<'a, S: MemorySource> Iterator for DebugSliceStructureIter<'a, S> {
    type Item = DebugStructure<'a, S>;

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
pub struct DebugSlice<'a, S: MemorySource> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo<'a, S>,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    length: u64,
    data_ptr: &'a unit_info::Pointer,
}

impl<'a, S: MemorySource> DebugSlice<'a, S> {
    pub fn base_type_iter(&self) -> Option<DebugSliceBaseTypeIter<'a, S>> {
        let Some(base_type) = self.info.base_type_from_kind(self.data_ptr.kind()) else {
            return None;
        };
        let Some(element_size) = self.info.size_from_kind(self.data_ptr.kind()) else {
            return None;
        };
        Some(DebugSliceBaseTypeIter {
            // unit: self.unit,
            info: self.info,
            location: self.location,
            offset: self.offset,
            length: self.length,
            current: 0,
            size: element_size,
            base_type,
        })
    }

    pub fn structure_iter(&self) -> Option<DebugSliceStructureIter<'a, S>> {
        let Some(structure) = self.info.structure_from_kind(self.data_ptr.kind()) else {
            return None;
        };
        let Some(element_size) = self.info.size_from_kind(self.data_ptr.kind()) else {
            println!("Couldn't iterate through a structure");
            return None;
        };
        Some(DebugSliceStructureIter {
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
pub struct DebugStructure<'a, S: MemorySource> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo<'a, S>,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    structure: &'a unit_info::Structure,
}

impl<'a, S: MemorySource> DebugStructure<'a, S> {
    pub fn member_named(&self, name: &str) -> Option<DebugStructureMember<'a, S>> {
        self.structure
            .member_named(name)
            .map(|structure_member| DebugStructureMember {
                unit: self.unit,
                info: self.info,
                location: self.location,
                offset: self.offset + structure_member.offset(),
                structure_member,
            })
    }

    /// Special case for Rust slices, which always have two members:
    /// a "data_ptr" and a "length".
    pub fn as_slice(&self) -> Option<DebugSlice<'a, S>> {
        if self.structure.members().len() != 2 {
            return None;
        }
        let length = self.member_named("length")?.base_type()?.as_u64()?;
        let data_ptr = self
            .member_named("data_ptr")?
            .pointer()?
            .follow_unless_null()?;
        Some(DebugSlice {
            unit: self.unit,
            info: self.info,
            location: data_ptr.location,
            offset: self.offset,
            length,
            data_ptr: data_ptr.pointer,
        })
    }

    pub fn location(&self) -> Option<unit_info::MemoryLocation> {
        self.location
    }
}

impl<'a, S: MemorySource> core::fmt::Debug for DebugStructure<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugStructure")
            .field("structure", &self.structure)
            .finish()
    }
}

impl<'a, S: MemorySource> core::ops::Deref for DebugStructure<'a, S> {
    type Target = unit_info::Structure;

    fn deref(&self) -> &Self::Target {
        self.structure
    }
}

/// Wrap a Pointer to include the unit that it came from
pub struct DebugPointer<'a, S: MemorySource> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo<'a, S>,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    pointer: &'a unit_info::Pointer,
}

impl<'a, S: MemorySource> DebugPointer<'a, S> {
    pub fn structure(&self) -> Option<DebugStructure<'a, S>> {
        self.info
            .structure_from_kind(self.pointer.kind())
            .map(|structure| DebugStructure {
                unit: self.unit,
                structure,
                location: self.location,
                offset: self.offset,
                info: self.info,
            })
    }

    pub fn follow_unless_null(self) -> Option<Self> {
        if let Some(new) = self.follow() {
            if new.location.map(|v| v != MemoryLocation(0)).unwrap_or(true) {
                return Some(new);
            }
        }
        None
    }

    pub fn follow(mut self) -> Option<Self> {
        let location = self.location?.0;
        let target = self.info.memory_source.read_u32(location.into()).ok()?;
        self.location = Some(MemoryLocation(target.into()));
        self.offset = StructOffset::new(0);
        Some(self)
    }

    /// Read a u8 from the specified offset
    pub fn read_u8(&self, offset: u64) -> Option<u8> {
        let location = self.location?.0 + offset;
        self.info.memory_source.read_u8(location.into()).ok()
    }
}

impl<'a, S: MemorySource> core::fmt::Debug for DebugPointer<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugPointer")
            .field("pointer", &self.pointer)
            .finish()
    }
}

impl<'a, S: MemorySource> core::ops::Deref for DebugPointer<'a, S> {
    type Target = unit_info::Pointer;

    fn deref(&self) -> &Self::Target {
        self.pointer
    }
}

/// Wrap an Enumeration to include the unit that it came from
pub struct DebugEnumerationVariant<'a, S: MemorySource> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo<'a, S>,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    variant: &'a unit_info::EnumerationVariant,
}

impl<'a, S: MemorySource> DebugEnumerationVariant<'a, S> {
    pub fn structure(&self) -> Option<DebugStructure<'a, S>> {
        self.info
            .structure_from_kind(self.variant.kind())
            .map(|structure| DebugStructure {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + self.variant.offset()),
                offset: self.offset + self.variant.offset(),
                structure,
            })
    }
}

impl<'a, S: MemorySource> core::fmt::Debug for DebugEnumerationVariant<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugEnumerationVariant")
            .field("variant", &self.variant)
            .finish()
    }
}

impl<'a, S: MemorySource> core::ops::Deref for DebugEnumerationVariant<'a, S> {
    type Target = unit_info::EnumerationVariant;

    fn deref(&self) -> &Self::Target {
        self.variant
    }
}

/// Wrap an Enumeration to include the unit that it came from
pub struct DebugEnumeration<'a, S: MemorySource> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo<'a, S>,
    location: Option<unit_info::MemoryLocation>,
    offset: unit_info::StructOffset,
    enumeration: &'a unit_info::Enumeration,
}

impl<'a, S: MemorySource> DebugEnumeration<'a, S> {
    pub fn variant_at(&self, index: usize) -> Option<DebugEnumerationVariant<'a, S>> {
        self.enumeration
            .variant_at(index)
            .map(|variant| DebugEnumerationVariant {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + variant.offset()),
                offset: self.offset + variant.offset(),
                variant,
            })
    }
    pub fn variant_named(&self, name: &str) -> Option<DebugEnumerationVariant<'a, S>> {
        self.enumeration
            .variant_named(name)
            .map(|variant| DebugEnumerationVariant {
                unit: self.unit,
                info: self.info,
                location: self.location.map(|l| l + variant.offset()),
                offset: self.offset + variant.offset(),
                variant,
            })
    }

    /// Returns the currently-selected variant, if one is available
    pub fn variant(&self) -> Option<DebugEnumerationVariant<'a, S>> {
        let address = self.location?.0;
        let discriminant_size = self.info.size_from_kind(self.discriminant_kind())?;
        let discriminant: u64 = match discriminant_size.0 {
            1 => self.info.memory_source.read_u8(address).ok()?.into(),
            2 => self.info.memory_source.read_u16(address).ok()?.into(),
            4 => self.info.memory_source.read_u32(address).ok()?.into(),
            8 => self.info.memory_source.read_u64(address).ok()?,
            _ => return None,
        };
        self.variant_at(discriminant as usize)
    }
}

impl<'a, S: MemorySource> core::fmt::Debug for DebugEnumeration<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugEnumeration")
            .field("enumeration", &self.enumeration)
            .finish()
    }
}

impl<'a, S: MemorySource> core::ops::Deref for DebugEnumeration<'a, S> {
    type Target = unit_info::Enumeration;

    fn deref(&self) -> &Self::Target {
        self.enumeration
    }
}

/// Wrap a Variable to include the unit that it came from
pub struct DebugVariable<'a, S: MemorySource> {
    unit: &'a unit_info::UnitInfo,
    info: &'a DebugInfo<'a, S>,
    variable: &'a unit_info::Variable,
}

impl<'a, S: MemorySource> DebugVariable<'a, S> {
    pub fn new(
        unit: &'a unit_info::UnitInfo,
        info: &'a DebugInfo<'a, S>,
        variable: &'a unit_info::Variable,
    ) -> Self {
        DebugVariable {
            unit,
            info,
            variable,
        }
    }

    pub fn structure(&self) -> Option<DebugStructure<'a, S>> {
        self.info
            .structure_from_kind(self.variable.kind())
            .map(|structure| DebugStructure {
                unit: self.unit,
                info: self.info,
                location: Some(self.variable.location()),
                offset: unit_info::StructOffset::new(0),
                structure,
            })
    }

    pub fn enumeration(&self) -> Option<DebugEnumeration<'a, S>> {
        self.info
            .enumeration_from_kind(self.variable.kind())
            .map(|enumeration| DebugEnumeration {
                unit: self.unit,
                info: self.info,
                location: Some(self.variable.location()),
                offset: unit_info::StructOffset::new(0),
                enumeration,
            })
    }

    pub fn array(&self) -> Option<DebugArray<'a, S>> {
        self.info
            .array_from_kind(self.variable.kind())
            .map(|array| DebugArray {
                unit: self.unit,
                info: self.info,
                location: Some(self.variable.location()),
                offset: unit_info::StructOffset::new(0),
                array,
            })
    }
}

impl<'a, S: MemorySource> core::ops::Deref for DebugVariable<'a, S> {
    type Target = unit_info::Variable;

    fn deref(&self) -> &Self::Target {
        self.variable
    }
}

impl<'a, S: MemorySource> core::fmt::Debug for DebugVariable<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugVariable")
            // .field("unit", &self.unit)
            .field("variable", &self.variable)
            .finish()
    }
}
