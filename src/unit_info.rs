use super::GimliReader;
use gimli::Reader;
use std::collections::HashMap;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
/// A location within the debug section
pub struct DebugItemOffset {
    offset: u64,
}

impl DebugItemOffset {
    pub fn from_unit_offset<'a>(
        offset: gimli::UnitOffset,
        unit_ref: gimli::UnitRef<'a, GimliReader>,
    ) -> Option<Self> {
        offset
            .to_debug_info_offset(&unit_ref.unit.header)
            .map(|offset| DebugItemOffset {
                offset: offset.0 as u64,
            })
    }
    pub fn from_debug_info_offset(offset: gimli::DebugInfoOffset) -> Self {
        DebugItemOffset {
            offset: offset.0 as u64,
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
/// A location within the running target
pub struct MemoryLocation(pub(crate) u64);

impl core::fmt::Display for MemoryLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{:08x}>", self.0)
    }
}

impl core::ops::Add<StructOffset> for MemoryLocation {
    type Output = Self;

    fn add(self, rhs: StructOffset) -> Self::Output {
        MemoryLocation(self.0 + rhs.0)
    }
}

impl core::ops::Mul<StructOffset> for MemoryLocation {
    type Output = Self;

    fn mul(self, rhs: StructOffset) -> Self::Output {
        MemoryLocation(self.0 * rhs.0)
    }
}

impl core::ops::AddAssign<StructOffset> for MemoryLocation {
    fn add_assign(&mut self, rhs: StructOffset) {
        self.0 += rhs.0
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
/// An offset from the start of the struct
pub struct StructOffset(pub(crate) u64);
impl StructOffset {
    pub fn new(offset: u64) -> Self {
        StructOffset(offset)
    }
}

impl core::ops::Add<StructOffset> for StructOffset {
    type Output = Self;

    fn add(self, rhs: StructOffset) -> Self::Output {
        StructOffset(self.0 + rhs.0)
    }
}

impl core::ops::Mul<StructOffset> for StructOffset {
    type Output = Self;

    fn mul(self, rhs: StructOffset) -> Self::Output {
        StructOffset(self.0 * rhs.0)
    }
}

impl core::fmt::Display for StructOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{:08x}>", self.0)
    }
}

#[derive(Debug)]
pub struct FileName(String);

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
/// An index into a vec
struct EntryIndex(usize);

impl core::fmt::Display for EntryIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{:08x}>", self.0)
    }
}

#[derive(Debug)]
pub struct StructureMember {
    name: Option<String>,
    kind: DebugItemOffset,
    offset: StructOffset,
}

impl StructureMember {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn kind(&self) -> DebugItemOffset {
        self.kind
    }

    pub fn offset(&self) -> StructOffset {
        self.offset
    }
}

pub struct Pointer {
    name: Option<String>,
    kind: DebugItemOffset,
}

impl Pointer {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    pub fn kind(&self) -> DebugItemOffset {
        self.kind
    }
}

impl core::fmt::Debug for Pointer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pointer")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .finish()
    }
}

pub struct BaseType {
    name: String,
    size: u64,
}

impl BaseType {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}

impl core::fmt::Debug for BaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BaseType")
            .field("name", &self.name)
            .field("size", &self.size)
            .finish()
    }
}

#[derive(Debug)]
pub struct Union {
    name: String,
    members: Vec<StructureMember>,
    size: u64,
}

impl Union {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn members(&self) -> &[StructureMember] {
        &self.members
    }

    pub fn member_named(&self, name: &str) -> Option<&StructureMember> {
        for member in &self.members {
            if member.name.as_deref() == Some(name) {
                return Some(member);
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct EnumerationVariant {
    name: String,
    discriminant: Option<u64>,
    kind: DebugItemOffset,
    offset: StructOffset,
}

impl EnumerationVariant {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn kind(&self) -> DebugItemOffset {
        self.kind
    }
    pub fn offset(&self) -> StructOffset {
        self.offset
    }
    pub fn discriminant(&self) -> Option<u64> {
        self.discriminant
    }
}

#[derive(Debug)]
pub struct Enumeration {
    name: String,
    discriminant_offset: StructOffset,
    discriminant_kind: DebugItemOffset,
    size: u64,
    variants: Vec<EnumerationVariant>,
}

impl Enumeration {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn variant_at(&self, index: usize) -> Option<&EnumerationVariant> {
        self.variants.get(index)
    }

    pub fn variant_named(&self, name: &str) -> Option<&EnumerationVariant> {
        for variant in &self.variants {
            if variant.name == name {
                return Some(variant);
            }
        }
        None
    }

    pub fn discriminant_offset(&self) -> StructOffset {
        self.discriminant_offset
    }

    pub fn discriminant_kind(&self) -> DebugItemOffset {
        self.discriminant_kind
    }
}

#[derive(Debug)]
/// Represents either a struct or an enum.
pub struct Structure {
    name: String,
    members: Vec<StructureMember>,
    size: u64,
    containing_type: Option<DebugItemOffset>,
}

impl Structure {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn members(&self) -> &[StructureMember] {
        &self.members
    }

    pub fn member_named(&self, name: &str) -> Option<&StructureMember> {
        for member in &self.members {
            if member.name.as_deref() == Some(name) {
                return Some(member);
            }
        }
        None
    }
    pub fn size(&self) -> u64 {
        self.size
    }
    pub fn containing_type(&self) -> Option<DebugItemOffset> {
        self.containing_type
    }
}

#[derive(Debug)]
pub struct Array {
    kind: DebugItemOffset,
    lower_bound: u64,
    count: usize,
}

impl Array {
    pub fn kind(&self) -> DebugItemOffset {
        self.kind
    }
    pub fn count(&self) -> usize {
        self.count
    }
    pub fn lower_bound(&self) -> u64 {
        self.lower_bound
    }
}

/// Arrays are stored as an array_type followed by a subrange_type. This contains
/// just the array_type.
struct PartialArray {
    kind: DebugItemOffset,
}

/// A tagthat describes the contents of the array
struct Subrange {
    lower_bound: u64,
    count: usize,
}

#[derive(Debug)]
pub struct Variable {
    name: String,
    kind: DebugItemOffset,
    location: MemoryLocation,
    linkage_name: Option<String>,
    line: Option<u64>,
    file: Option<FileName>,
}

impl Variable {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> DebugItemOffset {
        self.kind
    }

    pub fn location(&self) -> MemoryLocation {
        self.location
    }

    pub fn file(&self) -> Option<&str> {
        self.file.as_ref().map(|v| v.0.as_ref())
    }
    pub fn line(&self) -> Option<u64> {
        self.line
    }
}

pub struct SymbolCache {
    /// A list of all variables in this section
    variables: Vec<Variable>,

    /// A list of all structures in this section
    structures: Vec<Structure>,

    /// A list of all enumerations in this section
    enumerations: Vec<Enumeration>,

    /// A list of all arrays in this section
    arrays: Vec<Array>,

    /// A list of all pointers in this section
    pointers: Vec<Pointer>,

    /// A list of all base types in this section
    base_types: Vec<BaseType>,

    /// A list of all unions in this section
    unions: Vec<Union>,

    /// Pointers to variables by the variable's exported name
    variable_names: HashMap<String, EntryIndex>,

    /// Pointers to variables by the variable's demangled exported name
    demangled_variable_names: HashMap<String, EntryIndex>,

    /// Pointers from the variable's address to the variable
    variable_address: HashMap<DebugItemOffset, EntryIndex>,

    /// Pointers from the structure's offset to the structure
    structure_address: HashMap<DebugItemOffset, EntryIndex>,

    /// Pointers from the structure's offset to the enumeration
    enumeration_address: HashMap<DebugItemOffset, EntryIndex>,

    /// Pointers from the array's offset to the array
    array_address: HashMap<DebugItemOffset, EntryIndex>,

    /// Pointers from the pointer's offset to the pointer
    pointer_address: HashMap<DebugItemOffset, EntryIndex>,

    /// Pointers from the base type's offset to the base type
    base_type_address: HashMap<DebugItemOffset, EntryIndex>,

    /// Pointers from the union's offset to the union
    union_address: HashMap<DebugItemOffset, EntryIndex>,
}

/// A struct containing information about a single compilation unit.
pub struct UnitInfo {
    cache: SymbolCache,
}

impl UnitInfo {
    pub fn all_symbols(&self) -> Vec<DebugItemOffset> {
        self.cache
            .array_address
            .keys()
            .chain(self.cache.base_type_address.keys())
            .chain(self.cache.enumeration_address.keys())
            .chain(self.cache.pointer_address.keys())
            .chain(self.cache.structure_address.keys())
            .chain(self.cache.union_address.keys())
            .chain(self.cache.variable_address.keys())
            .map(|x| *x)
            .collect()
    }

    pub fn new(unit: gimli::Unit<GimliReader>, dwarf: &gimli::Dwarf<GimliReader>) -> Option<Self> {
        let unit_ref = unit.unit_ref(dwarf);
        let mut variables = vec![];
        let mut structures: Vec<Structure> = vec![];
        let mut enumerations = vec![];
        let mut arrays = vec![];
        let mut pointers = vec![];
        let mut base_types = vec![];
        let mut unions: Vec<Union> = vec![];
        let mut variable_names = HashMap::new();
        let mut demangled_variable_names = HashMap::new();

        let mut variable_address = HashMap::new();
        let mut structure_address = HashMap::new();
        let mut enumeration_address = HashMap::new();
        let mut array_address = HashMap::new();
        let mut pointer_address = HashMap::new();
        let mut base_type_address = HashMap::new();
        let mut union_address = HashMap::new();

        let mut array_in_progress: Option<(PartialArray, DebugItemOffset)> = None;
        let mut tag_parent_list = vec![];
        let mut last_structure_address: Option<DebugItemOffset> = None;

        let mut entries = unit_ref.entries();
        let mut depth = 0usize;
        while let Ok(Some((depth_delta, abbrev))) = entries.next_dfs() {
            if depth_delta < 0 {
                if depth_delta.abs() as usize > depth {
                    panic!(
                        "Depth went negative! Delta: {}  depth: {}",
                        depth_delta, depth
                    );
                }
                depth = depth.saturating_sub(depth_delta.abs() as usize);
            } else {
                depth = depth.saturating_add(depth_delta as usize);
            };

            tag_parent_list.truncate(depth);
            // Build the tag parent list up to the current depth.
            while tag_parent_list.len() <= depth {
                tag_parent_list.push(gimli::constants::DW_TAG_null);
            }
            tag_parent_list.pop();
            tag_parent_list.push(abbrev.tag());

            // for _ in 0..depth {
            //     print!("  ");
            // }
            // for tag in &tag_parent_list {
            //     print!("{}:", tag.static_string().unwrap_or("<unknown>"));
            // }
            // println!(
            //     "<{}> @ {:08x} {}",
            //     depth,
            //     abbrev.offset().0,
            //     abbrev.tag().static_string().unwrap_or("<unknown>")
            // );

            let parent_tag = *tag_parent_list
                .get(tag_parent_list.len().saturating_sub(2))
                .unwrap_or(&gimli::constants::DW_TAG_null);

            // if abbrev.tag() == gimli::constants::DW_TAG_member {
            //     println!(
            //         "Tag is member and parent tag is: {}",
            //         parent_tag.static_string().unwrap_or("<unknown>")
            //     );
            // }

            match abbrev.tag() {
                gimli::constants::DW_TAG_variable => {
                    let Some(variable) = parse_variable(abbrev.attrs(), unit_ref) else {
                        continue;
                    };

                    let Some(offset) = DebugItemOffset::from_unit_offset(abbrev.offset(), unit_ref)
                    else {
                        continue;
                    };

                    // If the name exists, add it to the name lookup table.
                    if let Some(linkage_name) = &variable.linkage_name {
                        assert!(variable_names
                            .insert(linkage_name.clone(), EntryIndex(variables.len()))
                            .is_none());
                        assert!(demangled_variable_names
                            .insert(
                                format!("{:#}", rustc_demangle::demangle(linkage_name)),
                                EntryIndex(variables.len()),
                            )
                            .is_none());
                    }
                    assert!(variable_address
                        .insert(offset, EntryIndex(variables.len()),)
                        .is_none());
                    variables.push(variable);
                }
                // This is actually an enum, not a struct. Convert it to an enum.
                gimli::constants::DW_TAG_variant_part
                    if parent_tag == gimli::constants::DW_TAG_structure_type =>
                {
                    let Some(structure) = structures.pop() else {
                        println!("Structure was NONE!");
                        continue;
                    };
                    // Remove the struct form the address and add it to the enumeration list
                    let last_structure_address = last_structure_address.take().unwrap();
                    assert!(structure_address.remove(&last_structure_address).is_some());
                    enumeration_address
                        .insert(last_structure_address, EntryIndex(enumerations.len()));
                    // TODO: Parse `discr` type. For now we just assume it's the first one.
                    enumerations.push(Enumeration {
                        name: structure.name,
                        discriminant_kind: DebugItemOffset::from_debug_info_offset(
                            gimli::DebugInfoOffset(0),
                        ),
                        discriminant_offset: StructOffset(0),
                        size: structure.size,
                        variants: vec![],
                    });
                }

                // Enum discriminant specification
                gimli::constants::DW_TAG_member
                    if parent_tag == gimli::constants::DW_TAG_variant_part =>
                {
                    if let Some(last_enum) = enumerations.last_mut() {
                        parse_enum_discriminant(abbrev.attrs(), last_enum, unit_ref);
                    }
                }

                // Enum variant ID
                gimli::constants::DW_TAG_variant
                    if parent_tag == gimli::constants::DW_TAG_variant_part =>
                {
                    let discriminant = parse_enum_variant(abbrev.attrs());
                    if let Some(last_enum) = enumerations.last_mut() {
                        last_enum.variants.push(EnumerationVariant {
                            name: String::new(),
                            discriminant,
                            kind: DebugItemOffset::from_debug_info_offset(gimli::DebugInfoOffset(
                                0,
                            )),
                            offset: StructOffset(0),
                        });
                    }
                }

                // Enum variant specification
                gimli::constants::DW_TAG_member
                    if parent_tag == gimli::constants::DW_TAG_variant =>
                {
                    if let Some(last_enum) = enumerations.last_mut() {
                        if let Some(last_variant) = last_enum.variants.last_mut() {
                            update_enum_variant_member(abbrev.attrs(), last_variant, unit_ref);
                        }
                    }
                }

                // Structure member
                gimli::constants::DW_TAG_member
                    if parent_tag == gimli::constants::DW_TAG_structure_type =>
                {
                    if let Some(member) = parse_structure_member(abbrev.attrs(), unit_ref) {
                        if let Some(last) = structures.last_mut() {
                            last.members.push(member);
                        }
                    }
                }

                // Union member
                gimli::constants::DW_TAG_member
                    if parent_tag == gimli::constants::DW_TAG_union_type =>
                {
                    if let Some(member) = parse_structure_member(abbrev.attrs(), unit_ref) {
                        if let Some(last) = unions.last_mut() {
                            last.members.push(member);
                        }
                    }
                }

                gimli::constants::DW_TAG_structure_type => {
                    let Some(structure) = parse_structure(abbrev.attrs(), unit_ref) else {
                        continue;
                    };
                    let Some(offset) = DebugItemOffset::from_unit_offset(abbrev.offset(), unit_ref)
                    else {
                        continue;
                    };
                    assert!(structure_address
                        .insert(offset, EntryIndex(structures.len()))
                        .is_none());
                    last_structure_address = Some(offset);
                    structures.push(structure);
                }

                gimli::constants::DW_TAG_union_type => {
                    let Some(new_union) = parse_union(abbrev.attrs(), unit_ref) else {
                        continue;
                    };
                    let Some(offset) = DebugItemOffset::from_unit_offset(abbrev.offset(), unit_ref)
                    else {
                        continue;
                    };
                    assert!(union_address
                        .insert(offset, EntryIndex(unions.len()))
                        .is_none());
                    last_structure_address = Some(offset);
                    unions.push(new_union);
                }

                gimli::constants::DW_TAG_array_type => {
                    let Some(offset) = DebugItemOffset::from_unit_offset(abbrev.offset(), unit_ref)
                    else {
                        continue;
                    };
                    array_in_progress = parse_array(abbrev.attrs(), unit_ref).map(|v| (v, offset));
                }
                gimli::constants::DW_TAG_subrange_type
                    if parent_tag == gimli::constants::DW_TAG_array_type =>
                {
                    let Some(subrange) = parse_subrange(abbrev.attrs()) else {
                        continue;
                    };
                    let Some((array_in_progress, offset)) = array_in_progress.take() else {
                        panic!("Got a subrange without an array in progress! Are there two subtypes? Or no array type?");
                    };
                    let array = Array {
                        kind: array_in_progress.kind,
                        lower_bound: subrange.lower_bound,
                        count: subrange.count,
                    };
                    assert!(array_address
                        .insert(offset, EntryIndex(arrays.len()))
                        .is_none());
                    arrays.push(array);
                }
                gimli::constants::DW_TAG_pointer_type => {
                    let Some(pointer) = parse_pointer(abbrev.attrs(), unit_ref) else {
                        continue;
                    };
                    let Some(offset) = abbrev.offset().to_debug_info_offset(&unit.header) else {
                        continue;
                    };
                    assert!(pointer_address
                        .insert(
                            DebugItemOffset::from_debug_info_offset(offset),
                            EntryIndex(pointers.len())
                        )
                        .is_none());
                    pointers.push(pointer);
                }

                gimli::constants::DW_TAG_base_type => {
                    let Some(base_type) = parse_base_type(abbrev.attrs(), unit_ref) else {
                        continue;
                    };
                    let Some(offset) = abbrev.offset().to_debug_info_offset(&unit.header) else {
                        continue;
                    };
                    assert!(base_type_address
                        .insert(
                            DebugItemOffset::from_debug_info_offset(offset),
                            EntryIndex(base_types.len())
                        )
                        .is_none());
                    base_types.push(base_type);
                }
                _tag => {
                    // println!(
                    //     "Unrecognized tag type: {}",
                    //     tag.static_string().unwrap_or("<unknown>")
                    // )
                }
            }
        }

        let cache = SymbolCache {
            variables,
            structures,
            enumerations,
            arrays,
            pointers,
            base_types,
            unions,
            variable_names,
            demangled_variable_names,
            variable_address,
            structure_address,
            enumeration_address,
            array_address,
            pointer_address,
            base_type_address,
            union_address,
        };

        Some(Self { cache })
    }

    pub fn variable_from_name(&self, name: &str) -> Option<&Variable> {
        self.cache
            .variable_names
            .get(name)
            .and_then(|addr| self.cache.variables.get(addr.0))
    }

    pub fn variable_from_demangled_name(&self, name: &str) -> Option<&Variable> {
        self.cache
            .demangled_variable_names
            .get(name)
            .and_then(|addr| self.cache.variables.get(addr.0))
    }

    pub fn variable_from_kind(&self, location: DebugItemOffset) -> Option<&Variable> {
        self.cache
            .variable_address
            .get(&location)
            .and_then(|addr| self.cache.variables.get(addr.0))
    }

    pub fn structure_from_kind(&self, location: DebugItemOffset) -> Option<&Structure> {
        self.cache
            .structure_address
            .get(&location)
            .and_then(|addr| self.cache.structures.get(addr.0))
    }

    pub fn enumeration_from_kind(&self, location: DebugItemOffset) -> Option<&Enumeration> {
        self.cache
            .enumeration_address
            .get(&location)
            .and_then(|addr| self.cache.enumerations.get(addr.0))
    }

    pub fn array_from_kind(&self, location: DebugItemOffset) -> Option<&Array> {
        self.cache
            .array_address
            .get(&location)
            .and_then(|addr| self.cache.arrays.get(addr.0))
    }

    pub fn pointer_from_kind(&self, location: DebugItemOffset) -> Option<&Pointer> {
        self.cache
            .pointer_address
            .get(&location)
            .and_then(|addr| self.cache.pointers.get(addr.0))
    }

    pub fn base_type_from_kind(&self, location: DebugItemOffset) -> Option<&BaseType> {
        self.cache
            .base_type_address
            .get(&location)
            .and_then(|addr| self.cache.base_types.get(addr.0))
    }

    pub fn union_from_kind(&self, location: DebugItemOffset) -> Option<&Union> {
        self.cache
            .union_address
            .get(&location)
            .and_then(|addr| self.cache.unions.get(addr.0))
    }

    pub fn size_from_kind(&self, location: DebugItemOffset) -> Option<StructOffset> {
        if let Some(val) = self
            .cache
            .structure_address
            .get(&location)
            .and_then(|addr| self.cache.structures.get(addr.0))
        {
            Some(StructOffset(val.size))
        } else if let Some(val) = self
            .cache
            .enumeration_address
            .get(&location)
            .and_then(|addr| self.cache.enumerations.get(addr.0))
        {
            Some(StructOffset(val.size))
        } else if let Some(_val) = self.cache.array_address.get(&location) {
            panic!("Unable to get size of array");
        } else if let Some(_val) = self
            .cache
            .pointer_address
            .get(&location)
            .and_then(|addr| self.cache.pointers.get(addr.0))
        {
            panic!("Unable to get size of pointer");
        } else if let Some(val) = self
            .cache
            .base_type_address
            .get(&location)
            .and_then(|addr| self.cache.base_types.get(addr.0))
        {
            Some(StructOffset(val.size))
        } else if let Some(val) = self
            .cache
            .union_address
            .get(&location)
            .and_then(|addr| self.cache.unions.get(addr.0))
        {
            Some(StructOffset(val.size))
        } else {
            println!(
                "Unknown kind @ {:08x} -- can't determine size",
                location.offset
            );
            None
        }
    }

    pub fn name_from_kind(&self, location: DebugItemOffset) -> Option<&str> {
        if let Some(val) = self
            .cache
            .structure_address
            .get(&location)
            .and_then(|addr| self.cache.structures.get(addr.0))
        {
            Some(val.name())
        } else if let Some(val) = self
            .cache
            .enumeration_address
            .get(&location)
            .and_then(|addr| self.cache.enumerations.get(addr.0))
        {
            Some(val.name())
        } else if let Some(_val) = self
            .cache
            .array_address
            .get(&location)
            .and_then(|addr| self.cache.arrays.get(addr.0))
        {
            panic!("Unable to get size of array");
        } else if let Some(val) = self
            .cache
            .pointer_address
            .get(&location)
            .and_then(|addr| self.cache.pointers.get(addr.0))
        {
            val.name()
        } else if let Some(val) = self
            .cache
            .base_type_address
            .get(&location)
            .and_then(|addr| self.cache.base_types.get(addr.0))
        {
            Some(val.name())
        } else if let Some(val) = self
            .cache
            .union_address
            .get(&location)
            .and_then(|addr| self.cache.unions.get(addr.0))
        {
            Some(val.name())
        } else {
            println!(
                "Unknown kind @ {:08x} -- can't determine name",
                location.offset
            );
            None
        }
    }
}

fn parse_string(
    attr_value: gimli::AttributeValue<GimliReader>,
    unit_ref: gimli::UnitRef<GimliReader>,
) -> Option<String> {
    let gimli::AttributeValue::DebugStrRef(offset) = attr_value else {
        return None;
    };
    let Ok(new_name) = unit_ref.string(offset) else {
        return None;
    };
    new_name.to_string_lossy().map(|v| v.to_string()).ok()
}

fn parse_type(
    attr: gimli::Attribute<GimliReader>,
    unit_ref: gimli::UnitRef<GimliReader>,
) -> Option<DebugItemOffset> {
    if let gimli::AttributeValue::UnitRef(offset) = attr.value() {
        DebugItemOffset::from_unit_offset(offset, unit_ref)
    } else if let gimli::AttributeValue::DebugInfoRef(val) = attr.value() {
        Some(DebugItemOffset::from_debug_info_offset(val))
    } else {
        panic!("Unknown type index: {:?}", attr.value());
    }
}

fn parse_offset(
    attr: gimli::Attribute<GimliReader>,
    unit_ref: gimli::UnitRef<GimliReader>,
) -> Option<StructOffset> {
    match attr.value() {
        gimli::AttributeValue::LocationListsRef(_v) => {
            // panic!(
            //     "Location lists are unhandled -- but value is located at {:08x}",
            //     v.0
            // );
            None
        }
        gimli::AttributeValue::Udata(offset_from_location) => {
            Some(StructOffset(offset_from_location))
        }
        gimli::AttributeValue::Exprloc(expression) => {
            let result =
                super::extract::evaluate_expression(expression, unit_ref.unit.encoding()).ok()?;
            use super::extract::{ExpressionResult, VariableLocation};
            let ExpressionResult::Location(VariableLocation::Address(address)) = result else {
                // print!("Couldn't evaluate expression: ");
                // super::dump::attribute(&attr, unit_ref).ok();
                // panic!("Result was {:?}", result);
                return None;
            };
            // println!("Variable located at {:08x?}", address);
            Some(StructOffset(address))
        }
        _ => {
            print!("Unsupported value:");
            super::dump::attribute(&attr, unit_ref).ok();
            panic!();
        }
    }
}

fn parse_location(
    attr: gimli::Attribute<GimliReader>,
    unit_ref: gimli::UnitRef<GimliReader>,
) -> Option<MemoryLocation> {
    parse_offset(attr, unit_ref).map(|v| MemoryLocation(v.0))
}

fn parse_filename(
    attr: gimli::Attribute<GimliReader>,
    unit_ref: gimli::UnitRef<GimliReader>,
) -> Option<FileName> {
    let unit = unit_ref.unit;
    let gimli::AttributeValue::FileIndex(file_index) = attr.value() else {
        return None;
    };
    if file_index == 0 && unit.header.version() <= 4 {
        return None;
    }
    let header = match unit.line_program {
        Some(ref program) => program.header(),
        None => return None,
    };
    let file = match header.file(file_index) {
        Some(file) => file,
        None => {
            println!("Unable to get header for file {}", file_index);
            return None;
        }
    };
    // print!(" ");
    let mut file_name = String::new();
    if let Some(directory) = file.directory(header) {
        let directory = unit_ref.attr_string(directory).ok()?;
        let directory = directory.to_string_lossy().ok()?;
        if file.directory_index() != 0 && !directory.starts_with('/') {
            if let Some(ref comp_dir) = unit.comp_dir {
                file_name.push_str(&format!("{}/", comp_dir.to_string_lossy().ok()?));
                // print!("{}/", comp_dir.to_string_lossy()?,);
            }
        }
        file_name.push_str(&format!("{}/", directory));
    }
    file_name.push_str(&format!(
        "{}",
        unit_ref
            .attr_string(file.path_name())
            .ok()?
            .to_string_lossy()
            .ok()?
    ));
    Some(FileName(file_name))
}

fn parse_variable(
    mut attrs: gimli::AttrsIter<GimliReader>,
    unit_ref: gimli::UnitRef<GimliReader>,
) -> Option<Variable> {
    let mut name = None;
    let mut kind = None;
    let mut location = None;
    let mut linkage_name = None;
    let mut line = None;
    let mut file = None;

    while let Ok(Some(attr)) = attrs.next() {
        match attr.name() {
            gimli::constants::DW_AT_name => name = parse_string(attr.value(), unit_ref),
            gimli::constants::DW_AT_type => kind = parse_type(attr, unit_ref),
            gimli::constants::DW_AT_decl_file => file = parse_filename(attr, unit_ref),
            gimli::constants::DW_AT_decl_line => line = attr.udata_value(),
            gimli::constants::DW_AT_linkage_name => {
                linkage_name = parse_string(attr.value(), unit_ref);
            }
            gimli::constants::DW_AT_location => {
                location = parse_location(attr, unit_ref);
            }
            _ => {
                // println!(
                //     "Unrecognized: {}",
                //     attr.name().static_string().unwrap_or("<unknown>")
                // );
            }
        }
    }

    if let Some(name) = name {
        if let Some(kind) = kind {
            if let Some(location) = location {
                return Some(Variable {
                    name,
                    kind,
                    location,
                    linkage_name,
                    line,
                    file,
                });
            }
        }
    }
    None
}

fn parse_structure(
    mut attrs: gimli::AttrsIter<GimliReader>,
    unit_ref: gimli::UnitRef<GimliReader>,
) -> Option<Structure> {
    let mut name = None;
    let mut size = None;
    let mut containing_type = None;
    while let Ok(Some(attr)) = attrs.next() {
        match attr.name() {
            gimli::constants::DW_AT_name => name = parse_string(attr.value(), unit_ref),
            gimli::constants::DW_AT_byte_size => size = attr.udata_value(),
            gimli::constants::DW_AT_alignment => {}
            gimli::constants::DW_AT_accessibility => {}
            gimli::constants::DW_AT_containing_type => containing_type = parse_type(attr, unit_ref),
            _ => {
                println!(
                    "Unrecognized struct field: {}",
                    attr.name().static_string().unwrap_or("<unknown>")
                );
            }
        }
    }
    if let Some(name) = name {
        if let Some(size) = size {
            return Some(Structure {
                members: vec![],
                name,
                size,
                containing_type,
            });
        }
    }
    None
}

fn parse_union(
    mut attrs: gimli::AttrsIter<GimliReader>,
    unit_ref: gimli::UnitRef<GimliReader>,
) -> Option<Union> {
    let mut name = None;
    let mut size = None;
    while let Ok(Some(attr)) = attrs.next() {
        match attr.name() {
            gimli::constants::DW_AT_name => name = parse_string(attr.value(), unit_ref),
            gimli::constants::DW_AT_byte_size => size = attr.udata_value(),
            gimli::constants::DW_AT_alignment => {}
            gimli::constants::DW_AT_accessibility => {}
            // gimli::constants::DW_AT_containing_type => containing_type = parse_type(attr, unit_ref),
            _ => {
                println!(
                    "Unrecognized union field: {}",
                    attr.name().static_string().unwrap_or("<unknown>")
                );
            }
        }
    }
    if let Some(name) = name {
        if let Some(size) = size {
            return Some(Union {
                members: vec![],
                name,
                size,
            });
        }
    }
    None
}
fn parse_structure_member(
    mut attrs: gimli::AttrsIter<GimliReader>,
    unit_ref: gimli::UnitRef<GimliReader>,
) -> Option<StructureMember> {
    let mut name = None;
    let mut kind = None;
    let mut offset = None;
    while let Ok(Some(attr)) = attrs.next() {
        match attr.name() {
            gimli::constants::DW_AT_name => name = parse_string(attr.value(), unit_ref),
            gimli::constants::DW_AT_type => kind = parse_type(attr, unit_ref),
            gimli::constants::DW_AT_data_member_location => offset = parse_offset(attr, unit_ref),
            gimli::constants::DW_AT_alignment => {}
            gimli::constants::DW_AT_accessibility => {}
            _ => {
                println!(
                    "Unrecognized struct member attr: {}",
                    attr.name().static_string().unwrap_or("<unknown>")
                );
            }
        }
    }
    let offset = offset.unwrap_or(StructOffset(0));
    if let Some(kind) = kind {
        return Some(StructureMember { name, kind, offset });
    }
    None
}

fn parse_enum_variant(mut attrs: gimli::AttrsIter<GimliReader>) -> Option<u64> {
    let mut discriminant = None;
    while let Ok(Some(attr)) = attrs.next() {
        match attr.name() {
            gimli::constants::DW_AT_discr_value => {
                discriminant = attr.udata_value();
            }
            _ => {
                panic!(
                    "Unrecognized enum variant attr: {}",
                    attr.name().static_string().unwrap_or("<unknown>")
                );
            }
        }
    }
    discriminant
}

fn update_enum_variant_member(
    mut attrs: gimli::AttrsIter<GimliReader>,
    variant: &mut EnumerationVariant,
    unit_ref: gimli::UnitRef<GimliReader>,
) {
    while let Ok(Some(attr)) = attrs.next() {
        match attr.name() {
            gimli::constants::DW_AT_name => {
                if let Some(name) = parse_string(attr.value(), unit_ref) {
                    variant.name = name;
                }
            }
            gimli::constants::DW_AT_type => {
                if let Some(kind) = parse_type(attr, unit_ref) {
                    variant.kind = kind
                }
            }
            gimli::constants::DW_AT_alignment => {}
            gimli::constants::DW_AT_data_member_location => {
                if let Some(offset) = parse_offset(attr, unit_ref) {
                    variant.offset = offset
                }
            }
            gimli::constants::DW_AT_decl_file => {}
            gimli::constants::DW_AT_decl_line => {}
            _ => {
                panic!(
                    "Unrecognized enum variant member attr: {}",
                    attr.name().static_string().unwrap_or("<unknown>")
                );
            }
        }
    }
}

fn parse_enum_discriminant(
    mut attrs: gimli::AttrsIter<GimliReader>,
    enumeration: &mut Enumeration,
    unit_ref: gimli::UnitRef<GimliReader>,
) {
    let mut kind = None;
    let mut offset = None;
    while let Ok(Some(attr)) = attrs.next() {
        match attr.name() {
            gimli::constants::DW_AT_type => kind = parse_type(attr, unit_ref),
            gimli::constants::DW_AT_data_member_location => offset = parse_offset(attr, unit_ref),
            gimli::constants::DW_AT_artificial => {}
            gimli::constants::DW_AT_alignment => {}
            gimli::constants::DW_AT_name => {}
            _ => {
                println!(
                    "Unrecognized discriminant attr: {}",
                    attr.name().static_string().unwrap_or("<unknown>")
                );
            }
        }
    }
    enumeration.discriminant_offset = offset.unwrap_or(StructOffset(0));
    if let Some(kind) = kind {
        enumeration.discriminant_kind = kind;
    }
}

fn parse_array(
    mut attrs: gimli::AttrsIter<GimliReader>,
    unit_ref: gimli::UnitRef<GimliReader>,
) -> Option<PartialArray> {
    let mut kind = None;
    while let Ok(Some(attr)) = attrs.next() {
        match attr.name() {
            gimli::constants::DW_AT_type => kind = parse_type(attr, unit_ref),
            _ => {
                println!(
                    "Unrecognized array attr: {}",
                    attr.name().static_string().unwrap_or("<unknown>")
                );
            }
        }
    }
    if let Some(kind) = kind {
        return Some(PartialArray { kind });
    }
    None
}

fn parse_subrange(mut attrs: gimli::AttrsIter<GimliReader>) -> Option<Subrange> {
    let mut lower_bound = None;
    let mut count = None;
    while let Ok(Some(attr)) = attrs.next() {
        match attr.name() {
            gimli::constants::DW_AT_type => {}
            gimli::constants::DW_AT_lower_bound => lower_bound = attr.udata_value(),
            gimli::constants::DW_AT_count => {
                count = attr.udata_value().map(|udata| udata as usize);
            }
            _ => {
                println!(
                    "Unrecognized subrange attr: {}",
                    attr.name().static_string().unwrap_or("<unknown>")
                );
            }
        }
    }
    if let Some(lower_bound) = lower_bound {
        if let Some(count) = count {
            return Some(Subrange { lower_bound, count });
        }
    }
    None
}

fn parse_pointer(
    mut attrs: gimli::AttrsIter<GimliReader>,
    unit_ref: gimli::UnitRef<GimliReader>,
) -> Option<Pointer> {
    let mut name = None;
    let mut kind = None;
    while let Ok(Some(attr)) = attrs.next() {
        match attr.name() {
            gimli::constants::DW_AT_type => kind = parse_type(attr, unit_ref),
            gimli::constants::DW_AT_name => name = parse_string(attr.value(), unit_ref),
            gimli::constants::DW_AT_address_class => {}
            _ => {
                panic!(
                    "Unexpected pointer attr: {:?}",
                    attr.name().static_string().unwrap_or("<unknown>")
                );
            }
        }
    }
    kind.map(|kind| Pointer { name, kind })
}

fn parse_base_type(
    mut attrs: gimli::AttrsIter<GimliReader>,
    unit_ref: gimli::UnitRef<GimliReader>,
) -> Option<BaseType> {
    let mut name = None;
    let mut size = None;
    while let Ok(Some(attr)) = attrs.next() {
        match attr.name() {
            gimli::constants::DW_AT_name => name = parse_string(attr.value(), unit_ref),
            gimli::constants::DW_AT_byte_size => size = attr.udata_value(),
            gimli::constants::DW_AT_encoding => {}
            _ => {
                panic!(
                    "Unexpected base_type attr: {:?}",
                    attr.name().static_string().unwrap_or("<unknown>")
                );
            }
        }
    }
    if let Some(name) = name {
        if let Some(size) = size {
            return Some(BaseType { name, size });
        }
    }
    None
}
