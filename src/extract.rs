use crate::GimliReader;
use gimli::{Endianity, EvaluationResult, Location};

#[derive(Debug)]
pub enum ExtractError {
    WarnAndContinue { message: String },
    GimliError(gimli::Error),
    UnknownVariable,
}

impl From<gimli::Error> for ExtractError {
    fn from(value: gimli::Error) -> Self {
        ExtractError::GimliError(value)
    }
}

/// The result of `UnitInfo::evaluate_expression()` can be the value of a variable, or a memory location.
#[derive(Debug)]
pub(crate) enum ExpressionResult {
    #[allow(dead_code)]
    Value(u64),
    Location(VariableLocation),
}

// /// A [Variable] will have either a valid value, or some reason why a value could not be constructed.
// /// - If we encounter expected errors, they will be displayed to the user as defined below.
// /// - If we encounter unexpected errors, they will be treated as proper errors and will propagated
// ///   to the calling process as an `Err()`
// #[derive(Clone, Debug, PartialEq, Eq, Default)]
// pub enum VariableValue {
//     /// A valid value of this variable
//     Valid(String),
//     /// Notify the user that we encountered a problem correctly resolving the variable.
//     /// - The variable will be visible to the user, as will the other field of the variable.
//     /// - The contained warning message will be displayed to the user.
//     /// - The debugger will not attempt to resolve additional fields or children of this variable.
//     Error(String),
//     /// The value has not been set. This could be because ...
//     /// - It is too early in the process to have discovered its value, or ...
//     /// - The variable cannot have a stored value, e.g. a `struct`. In this case, please use
//     ///   `Variable::get_value` to infer a human readable value from the value of the struct's fields.
//     #[default]
//     Empty,
// }

// impl std::fmt::Display for VariableValue {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             VariableValue::Valid(value) => value.fmt(f),
//             VariableValue::Error(error) => write!(f, "< {error} >"),
//             VariableValue::Empty => write!(
//                 f,
//                 "Value not set. Please use Variable::get_value() to infer a human readable variable value"
//             ),
//         }
//     }
// }

// impl VariableValue {
//     /// Returns `true` if the variable resolver did not encounter an error, `false` otherwise.
//     pub fn is_valid(&self) -> bool {
//         !matches!(self, VariableValue::Error(_))
//     }

//     /// Returns `true` if no value or error is present, `false` otherwise.
//     pub fn is_empty(&self) -> bool {
//         matches!(self, VariableValue::Empty)
//     }
// }

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum VariableLocation {
    /// Location of the variable is not known. This means that it has not been evaluated yet.
    #[default]
    Unknown,
    /// The variable does not have a location currently, probably due to optimisations.
    Unavailable,
    /// The variable can be found in memory, at this address.
    Address(u64),
    /// The value of the variable is directly available.
    Value,
    /// There was an error evaluating the variable location.
    Error(String),
    /// Support for handling the location of this variable is not (yet) implemented.
    Unsupported(String),
}

impl VariableLocation {
    /// Return the memory address, if available. Otherwise an error is returned.
    pub fn memory_address(&self) -> Result<u64, ExtractError> {
        match self {
            VariableLocation::Address(address) => Ok(*address),
            other => Err(ExtractError::WarnAndContinue {
                message: format!("Variable does not have a memory location: location={other:?}"),
            }),
        }
    }

    /// Check if the location is valid, ie. not an error, unsupported, or unavailable.
    pub fn valid(&self) -> bool {
        match self {
            VariableLocation::Address(_) | VariableLocation::Value | VariableLocation::Unknown => {
                true
            }
            _other => false,
        }
    }
}

impl std::fmt::Display for VariableLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariableLocation::Unknown => "<unknown value>".fmt(f),
            VariableLocation::Unavailable => "<value not available>".fmt(f),
            VariableLocation::Address(address) => write!(f, "{address:#010X}"),
            VariableLocation::Value => "<not applicable - statically stored value>".fmt(f),
            VariableLocation::Error(error) => error.fmt(f),
            VariableLocation::Unsupported(reason) => reason.fmt(f),
        }
    }
}

// /// If a DW_AT_byte_size attribute exists, return the u64 value, otherwise (including errors) return None
// pub(crate) fn extract_byte_size(node_die: &DebuggingInformationEntry<GimliReader>) -> Option<u64> {
//     match node_die.attr(gimli::DW_AT_byte_size) {
//         Ok(Some(byte_size_attr)) => match byte_size_attr.value() {
//             AttributeValue::Udata(byte_size) => Some(byte_size),
//             AttributeValue::Data1(byte_size) => Some(byte_size as u64),
//             AttributeValue::Data2(byte_size) => Some(byte_size as u64),
//             AttributeValue::Data4(byte_size) => Some(byte_size as u64),
//             AttributeValue::Data8(byte_size) => Some(byte_size),
//             other => {
//                 eprintln!("Unimplemented: DW_AT_byte_size value: {other:?}");
//                 None
//             }
//         },
//         Ok(None) => None,
//         Err(error) => {
//             eprintln!(
//                 "Failed to extract byte_size: {error:?} for debug_entry {:?}",
//                 node_die.tag().static_string()
//             );
//             None
//         }
//     }
// }

// pub(crate) fn extract_line(attribute_value: AttributeValue<GimliReader>) -> Option<u64> {
//     match attribute_value {
//         AttributeValue::Udata(line) => Some(line),
//         _ => None,
//     }
// }

// /// If file information is available, it returns `Some(directory:PathBuf, file_name:String)`, otherwise `None`.
// pub(crate) fn extract_file(
//     // _debug_info: &DebugInfo,
//     _unit: &gimli::Unit<GimliReader>,
//     _attribute_value: AttributeValue<GimliReader>,
// ) -> Option<(TypedPathBuf, String)> {
//     // match attribute_value {
//     //     AttributeValue::FileIndex(index) => {
//     //         if let Some((Some(file), Some(path))) = debug_info.find_file_and_directory(unit, index)
//     //         {
//     //             Some((path, file))
//     //         } else {
//     //             eprintln!("Unable to extract file or path from {:?}.", attribute_value);
//     //             None
//     //         }
//     //     }
//     //     other => {
//     //         eprintln!(
//     //             "Unable to extract file information from attribute value {:?}: Not implemented.",
//     //             other
//     //         );
//     //         None
//     //     }
//     // }
//     None
// }

/// Tries to get the result of a DWARF expression in the form of a Piece.
pub(crate) fn expression_to_piece<ENDIAN: Endianity>(
    expression: gimli::Expression<GimliReader<ENDIAN>>,
    encoding: gimli::Encoding,
) -> Result<Vec<gimli::Piece<GimliReader<ENDIAN>, usize>>, ExtractError> {
    let mut evaluation = expression.evaluation(encoding);
    let mut result = evaluation.evaluate()?;

    loop {
        result = match result {
            EvaluationResult::Complete => return Ok(evaluation.result()),
            // EvaluationResult::RequiresMemory { address, size, .. } => {
            //     read_memory(size, memory, address, &mut evaluation)?
            // }
            // EvaluationResult::RequiresFrameBase => {
            //     provide_frame_base(frame_info.frame_base, &mut evaluation)?
            // }
            // EvaluationResult::RequiresRegister {
            //     register,
            //     base_type,
            // } => provide_register(frame_info.registers, register, base_type, &mut evaluation)?,
            EvaluationResult::RequiresRelocatedAddress(address_index) => {
                // The address_index as an offset from 0, so just pass it into the next step.
                evaluation.resume_with_relocated_address(address_index)?
            }
            // EvaluationResult::RequiresCallFrameCfa => {
            //     provide_cfa(frame_info.canonical_frame_address, &mut evaluation)?
            // }
            unimplemented_expression => {
                return Err(ExtractError::WarnAndContinue {
                    message: format!("Unimplemented: Expressions that include {unimplemented_expression:?} are not currently supported."
                )});
            }
        }
    }
}

/// Evaluate a [`gimli::Expression`] as a valid memory location.
/// Return values are implemented as follows:
/// - `Result<_, ExtractError>`: This happens when we encounter an error we did not expect, and will propagate upwards until the debugger request is failed. NOT GRACEFUL, and should be avoided.
/// - `Result<ExpressionResult::Value(),_>`: The value is statically stored in the binary, and can be returned, and has no relevant memory location.
/// - `Result<ExpressionResult::Location(),_>`: One of the variants of VariableLocation, and needs to be interpreted for handling the 'expected' errors we encounter during evaluation.
pub(crate) fn evaluate_expression<ENDIAN: Endianity>(
    expression: gimli::Expression<GimliReader<ENDIAN>>,
    encoding: gimli::Encoding,
) -> Result<ExpressionResult, ExtractError> {
    fn evaluate_address(address: u64) -> ExpressionResult {
        let location = if address >= u32::MAX as u64
        /*&& !memory.supports_native_64bit_access()*/
        {
            VariableLocation::Error(format!("The memory location for this variable value ({:#010X}) is invalid. Please report this as a bug.", address))
        } else {
            VariableLocation::Address(address)
        };
        ExpressionResult::Location(location)
    }

    let pieces = expression_to_piece(expression, encoding)?;

    if pieces.is_empty() {
        return Ok(ExpressionResult::Location(VariableLocation::Error(
            "Error: expr_to_piece() returned 0 results".to_string(),
        )));
    }
    if pieces.len() > 1 {
        return Ok(ExpressionResult::Location(VariableLocation::Error(
            "<unsupported memory implementation>".to_string(),
        )));
    }

    let result = match &pieces[0].location {
        Location::Empty => {
            // This means the value was optimized away.
            ExpressionResult::Location(VariableLocation::Unavailable)
        }
        Location::Address { address: 0 } => {
            let error = "The value of this variable may have been optimized out of the debug info, by the compiler.".to_string();
            ExpressionResult::Location(VariableLocation::Error(error))
        }
        Location::Address { address } => evaluate_address(*address),
        Location::Value { value } => value.to_u64(u64::MAX).map(ExpressionResult::Value)?,
        // Location::Register { register } => {
        //     if let Some(address) = frame_info
        //         .registers
        //         .get_register_by_dwarf_id(register.0)
        //         .and_then(|register| register.value)
        //     {
        //         match address.try_into() {
        //             Ok(address) => evaluate_address(address),
        //             Err(error) => ExpressionResult::Location(VariableLocation::Error(format!(
        //                 "Error: Cannot convert register value to location address: {error:?}"
        //             ))),
        //         }
        //     } else {
        //         ExpressionResult::Location(VariableLocation::Error(format!(
        //             "Error: Cannot resolve register: {register:?}"
        //         )))
        //     }
        // }
        l => ExpressionResult::Location(VariableLocation::Error(format!(
            "Unimplemented: extract_location() found a location type: {:.100}",
            format!("{l:?}")
        ))),
    };

    Ok(result)
}

// /// - Find the location using either DW_AT_location, DW_AT_data_member_location, or DW_AT_frame_base attribute.
// ///
// /// Return values are implemented as follows:
// /// - `Result<_, ExtractError>`: This happens when we encounter an error we did not expect, and will propagate upwards until the debugger request is failed. **NOT GRACEFUL**, and should be avoided.
// /// - `Result<ExpressionResult::Value(),_>`: The value is statically stored in the binary, and can be returned, and has no relevant memory location.
// /// - `Result<ExpressionResult::Location(),_>`: One of the variants of VariableLocation, and needs to be interpreted for handling the 'expected' errors we encounter during evaluation.
// pub(crate) fn extract_location(
//     debug_info: &DebugInfo,
//     node_die: &gimli::DebuggingInformationEntry<GimliReader>,
//     parent_location: &VariableLocation,
//     memory: &mut dyn MemoryInterface,
//     frame_info: StackFrameInfo<'_>,
// ) -> Result<ExpressionResult, ExtractError> {
//     trait ResultExt {
//         /// Turns UnwindIncompleteResults into Unavailable locations
//         fn convert_incomplete(self) -> Result<ExpressionResult, ExtractError>;
//     }

//     impl ResultExt for Result<ExpressionResult, ExtractError> {
//         fn convert_incomplete(self) -> Result<ExpressionResult, ExtractError> {
//             match self {
//                 Ok(result) => Ok(result),
//                 Err(ExtractError::WarnAndContinue { message }) => {
//                     tracing::warn!("UnwindIncompleteResults: {:?}", message);
//                     Ok(ExpressionResult::Location(VariableLocation::Unavailable))
//                 }
//                 e => e,
//             }
//         }
//     }

//     let mut attrs = node_die.attrs();
//     while let Ok(Some(attr)) = attrs.next() {
//         let result = match attr.name() {
//                 gimli::DW_AT_location
//                 | gimli::DW_AT_frame_base
//                 | gimli::DW_AT_data_member_location => match attr.value() {
//                     gimli::AttributeValue::Exprloc(expression) => evaluate_expression(memory, expression, frame_info)
//                         .convert_incomplete()?,

//                     gimli::AttributeValue::Udata(offset_from_location) => {
//                         let location = if let VariableLocation::Address(address) = parent_location {
//                             let Some(location) = address.checked_add(offset_from_location) else {
//                                 return Err(ExtractError::WarnAndContinue {
//                                     message: "Overflow calculating variable address"
//                                         .to_string(),
//                                 });
//                             };

//                             VariableLocation::Address(location)
//                         } else {
//                             parent_location.clone()
//                         };

//                         ExpressionResult::Location(location)
//                     }

//                     gimli::AttributeValue::LocationListsRef(location_list_offset) => self
//                         .evaluate_location_list_ref(
//                             debug_info,
//                             location_list_offset,
//                             frame_info,
//                             memory,
//                         )
//                         .convert_incomplete()?,

//                     other_attribute_value => {
//                         ExpressionResult::Location(VariableLocation::Unsupported(format!(
//                             "Unimplemented: extract_location() Could not extract location from: {:.100}",
//                             format!("{other_attribute_value:?}")
//                         )))
//                     }
//                 },

//                 gimli::DW_AT_address_class => {
//                     let location = match attr.value() {
//                         gimli::AttributeValue::AddressClass(gimli::DwAddr(0)) => {
//                             // We pass on the location of the parent, which will later to be used along with DW_AT_data_member_location to calculate the location of this variable.
//                             parent_location.clone()
//                         }
//                         gimli::AttributeValue::AddressClass(address_class) => {
//                             VariableLocation::Unsupported(format!(
//                                 "Unimplemented: extract_location() found unsupported DW_AT_address_class(gimli::DwAddr({address_class:?}))"
//                             ))
//                         }
//                         other_attribute_value => {
//                             VariableLocation::Unsupported(format!(
//                                 "Unimplemented: extract_location() found invalid DW_AT_address_class: {:.100}",
//                                 format!("{other_attribute_value:?}")
//                             ))
//                         }
//                     };

//                     ExpressionResult::Location(location)
//                 }

//                 _other_attributes => {
//                     // These will be handled elsewhere.
//                     continue;
//                 }
//             };

//         return Ok(result);
//     }

//     // If we get here, we did not find a location attribute, then leave the value as Unknown.
//     Ok(ExpressionResult::Location(VariableLocation::Unknown))
// }

// pub(crate) fn extract_location(
//     attrs: &[gimli::Attribute<GimliReader>],
//     unit: gimli::UnitRef<GimliReader>,
// ) -> Result<ExpressionResult, ExtractError> {
//     trait ResultExt {
//         /// Turns UnwindIncompleteResults into Unavailable locations
//         fn convert_incomplete(self) -> Result<ExpressionResult, ExtractError>;
//     }

//     impl ResultExt for Result<ExpressionResult, ExtractError> {
//         fn convert_incomplete(self) -> Result<ExpressionResult, ExtractError> {
//             match self {
//                 Ok(result) => Ok(result),
//                 Err(ExtractError::WarnAndContinue { .. }) => {
//                     // tracing::warn!("UnwindIncompleteResults: {:?}", message);
//                     Ok(ExpressionResult::Location(VariableLocation::Unavailable))
//                 }
//                 e => e,
//             }
//         }
//     }

//     for attr in attrs {
//         let result = match attr.name() {
//             gimli::DW_AT_location | gimli::DW_AT_frame_base | gimli::DW_AT_data_member_location => {
//                 match attr.value() {
//                     gimli::AttributeValue::Exprloc(expression) => {
//                         evaluate_expression(expression, unit.unit.encoding())
//                             .convert_incomplete()?
//                     }

//                     // gimli::AttributeValue::Udata(offset_from_location) => {
//                     //     let location = if let VariableLocation::Address(address) = parent_location {
//                     //         let Some(location) = address.checked_add(offset_from_location) else {
//                     //             return Err(ExtractError::WarnAndContinue {
//                     //                 message: "Overflow calculating variable address".to_string(),
//                     //             });
//                     //         };

//                     //         VariableLocation::Address(location)
//                     //     } else {
//                     //         parent_location.clone()
//                     //     };

//                     //     ExpressionResult::Location(location)
//                     // }

//                     // gimli::AttributeValue::LocationListsRef(location_list_offset) => self
//                     //     .evaluate_location_list_ref(
//                     //         debug_info,
//                     //         location_list_offset,
//                     //         frame_info,
//                     //         memory,
//                     //     )
//                     //     .convert_incomplete()?,
//                     other_attribute_value => {
//                         ExpressionResult::Location(VariableLocation::Unsupported(format!(
//                     "Unimplemented: extract_location() Could not extract location from: {:.100}",
//                     format!("{other_attribute_value:?}")
//                 )))
//                     }
//                 }
//             }

//             gimli::DW_AT_address_class => {
//                 let location = match attr.value() {
//                 // gimli::AttributeValue::AddressClass(gimli::DwAddr(0)) => {
//                 //     // We pass on the location of the parent, which will later to be used along with DW_AT_data_member_location to calculate the location of this variable.
//                 //     parent_location.clone()
//                 // }
//                 gimli::AttributeValue::AddressClass(address_class) => {
//                     VariableLocation::Unsupported(format!(
//                         "Unimplemented: extract_location() found unsupported DW_AT_address_class(gimli::DwAddr({address_class:?}))"
//                     ))
//                 }
//                 other_attribute_value => {
//                     VariableLocation::Unsupported(format!(
//                         "Unimplemented: extract_location() found invalid DW_AT_address_class: {:.100}",
//                         format!("{other_attribute_value:?}")
//                     ))
//                 }
//             };

//                 ExpressionResult::Location(location)
//             }

//             _other_attributes => {
//                 // These will be handled elsewhere.
//                 continue;
//             }
//         };
//         return Ok(result);
//     }
//     Ok(ExpressionResult::Location(VariableLocation::Unknown))
// }
