/*
 * Copyright Cedar Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! This module contains the diagnostics (i.e., errors and warnings) that are
//! returned by the validator.

use miette::Diagnostic;
use thiserror::Error;
use validation_errors::UnrecognizedActionIdHelp;

use std::collections::BTreeSet;

use cedar_policy_core::ast::{EntityType, Expr, PolicyID};
use cedar_policy_core::parser::Loc;

use crate::types::{EntityLUB, Type};

pub mod validation_errors;
pub mod validation_warnings;

/// Contains the result of policy validation. The result includes the list of
/// issues found by validation and whether validation succeeds or fails.
/// Validation succeeds if there are no fatal errors. There may still be
/// non-fatal warnings present when validation passes.
#[derive(Debug)]
pub struct ValidationResult {
    validation_errors: Vec<ValidationError>,
    validation_warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    /// Create a new `ValidationResult` with these errors and warnings.
    /// Empty iterators are allowed for either or both arguments.
    pub fn new(
        errors: impl IntoIterator<Item = ValidationError>,
        warnings: impl IntoIterator<Item = ValidationWarning>,
    ) -> Self {
        Self {
            validation_errors: errors.into_iter().collect(),
            validation_warnings: warnings.into_iter().collect(),
        }
    }

    /// True when validation passes. There are no errors, but there may be
    /// non-fatal warnings.
    pub fn validation_passed(&self) -> bool {
        self.validation_errors.is_empty()
    }

    /// Get an iterator over the errors found by the validator.
    pub fn validation_errors(&self) -> impl Iterator<Item = &ValidationError> {
        self.validation_errors.iter()
    }

    /// Get an iterator over the warnings found by the validator.
    pub fn validation_warnings(&self) -> impl Iterator<Item = &ValidationWarning> {
        self.validation_warnings.iter()
    }

    /// Get an iterator over the errors and warnings found by the validator.
    pub fn into_errors_and_warnings(
        self,
    ) -> (
        impl Iterator<Item = ValidationError>,
        impl Iterator<Item = ValidationWarning>,
    ) {
        (
            self.validation_errors.into_iter(),
            self.validation_warnings.into_iter(),
        )
    }
}

/// An error generated by the validator when it finds a potential problem in a
/// policy. The error contains a enumeration that specifies the kind of problem,
/// and provides details specific to that kind of problem. The error also records
/// where the problem was encountered.
//
// This is NOT a publicly exported error type.
#[derive(Clone, Debug, Diagnostic, Error, Hash, Eq, PartialEq)]
pub enum ValidationError {
    /// A policy contains an entity type that is not declared in the schema.
    #[error(transparent)]
    #[diagnostic(transparent)]
    UnrecognizedEntityType(#[from] validation_errors::UnrecognizedEntityType),
    /// A policy contains an action that is not declared in the schema.
    #[error(transparent)]
    #[diagnostic(transparent)]
    UnrecognizedActionId(#[from] validation_errors::UnrecognizedActionId),
    /// There is no action satisfying the action scope constraint that can be
    /// applied to a principal and resources that both satisfy their respective
    /// scope conditions.
    #[error(transparent)]
    #[diagnostic(transparent)]
    InvalidActionApplication(#[from] validation_errors::InvalidActionApplication),
    /// The typechecker expected to see a subtype of one of the types in
    /// `expected`, but saw `actual`.
    #[error(transparent)]
    #[diagnostic(transparent)]
    UnexpectedType(#[from] validation_errors::UnexpectedType),
    /// The typechecker could not compute a least upper bound for `types`.
    #[error(transparent)]
    #[diagnostic(transparent)]
    IncompatibleTypes(#[from] validation_errors::IncompatibleTypes),
    /// The typechecker detected an access to a record or entity attribute
    /// that it could not statically guarantee would be present.
    #[error(transparent)]
    #[diagnostic(transparent)]
    UnsafeAttributeAccess(#[from] validation_errors::UnsafeAttributeAccess),
    /// The typechecker could not conclude that an access to an optional
    /// attribute was safe.
    #[error(transparent)]
    #[diagnostic(transparent)]
    UnsafeOptionalAttributeAccess(#[from] validation_errors::UnsafeOptionalAttributeAccess),
    /// The typechecker could not conclude that an access to a tag was safe.
    #[error(transparent)]
    #[diagnostic(transparent)]
    UnsafeTagAccess(#[from] validation_errors::UnsafeTagAccess),
    /// `.getTag()` on an entity type which cannot have tags according to the schema.
    #[error(transparent)]
    #[diagnostic(transparent)]
    NoTagsAllowed(#[from] validation_errors::NoTagsAllowed),
    /// Undefined extension function.
    #[error(transparent)]
    #[diagnostic(transparent)]
    UndefinedFunction(#[from] validation_errors::UndefinedFunction),
    /// Incorrect number of arguments in an extension function application.
    #[error(transparent)]
    #[diagnostic(transparent)]
    WrongNumberArguments(#[from] validation_errors::WrongNumberArguments),
    /// Incorrect call style in an extension function application.
    /// Error returned by custom extension function argument validation
    #[diagnostic(transparent)]
    #[error(transparent)]
    FunctionArgumentValidation(#[from] validation_errors::FunctionArgumentValidation),
    /// The policy uses an empty set literal in a way that is forbidden
    #[diagnostic(transparent)]
    #[error(transparent)]
    EmptySetForbidden(#[from] validation_errors::EmptySetForbidden),
    /// The policy passes a non-literal to an extension constructor, which is
    /// forbidden in strict validation
    #[diagnostic(transparent)]
    #[error(transparent)]
    NonLitExtConstructor(#[from] validation_errors::NonLitExtConstructor),
    /// To pass strict validation a policy cannot contain an `in` expression
    /// where the entity type on the left might not be able to be a member of
    /// the entity type on the right.
    #[error(transparent)]
    #[diagnostic(transparent)]
    HierarchyNotRespected(#[from] validation_errors::HierarchyNotRespected),
    /// Returned when an internal invariant is violated (should not happen; if
    /// this is ever returned, please file an issue)
    #[error(transparent)]
    #[diagnostic(transparent)]
    InternalInvariantViolation(#[from] validation_errors::InternalInvariantViolation),
    #[cfg(feature = "level-validate")]
    /// If a entity dereference level was provided, the policies cannot deref
    /// more than `level` hops away from PARX
    #[error(transparent)]
    #[diagnostic(transparent)]
    EntityDerefLevelViolation(#[from] validation_errors::EntityDerefLevelViolation),
}

impl ValidationError {
    pub(crate) fn unrecognized_entity_type(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        actual_entity_type: String,
        suggested_entity_type: Option<String>,
    ) -> Self {
        validation_errors::UnrecognizedEntityType {
            source_loc,
            policy_id,
            actual_entity_type,
            suggested_entity_type,
        }
        .into()
    }

    pub(crate) fn unrecognized_action_id(
        source_loc: Option<Loc>,

        policy_id: PolicyID,
        actual_action_id: String,
        hint: Option<UnrecognizedActionIdHelp>,
    ) -> Self {
        validation_errors::UnrecognizedActionId {
            source_loc,
            policy_id,
            actual_action_id,
            hint,
        }
        .into()
    }

    pub(crate) fn invalid_action_application(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        would_in_fix_principal: bool,
        would_in_fix_resource: bool,
    ) -> Self {
        validation_errors::InvalidActionApplication {
            source_loc,
            policy_id,
            would_in_fix_principal,
            would_in_fix_resource,
        }
        .into()
    }

    /// Construct a type error for when an unexpected type occurs in an expression.
    pub(crate) fn expected_one_of_types(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        expected: Vec<Type>,
        actual: Type,
        help: Option<validation_errors::UnexpectedTypeHelp>,
    ) -> Self {
        validation_errors::UnexpectedType {
            source_loc,
            policy_id,
            expected,
            actual,
            help,
        }
        .into()
    }

    /// Construct a type error for when a least upper bound cannot be found for
    /// a collection of types.
    pub(crate) fn incompatible_types(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        types: impl IntoIterator<Item = Type>,
        hint: validation_errors::LubHelp,
        context: validation_errors::LubContext,
    ) -> Self {
        validation_errors::IncompatibleTypes {
            source_loc,
            policy_id,
            types: types.into_iter().collect::<BTreeSet<_>>(),
            hint,
            context,
        }
        .into()
    }

    pub(crate) fn unsafe_attribute_access(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        attribute_access: validation_errors::AttributeAccess,
        suggestion: Option<String>,
        may_exist: bool,
    ) -> Self {
        validation_errors::UnsafeAttributeAccess {
            source_loc,
            policy_id,
            attribute_access,
            suggestion,
            may_exist,
        }
        .into()
    }

    pub(crate) fn unsafe_optional_attribute_access(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        attribute_access: validation_errors::AttributeAccess,
    ) -> Self {
        validation_errors::UnsafeOptionalAttributeAccess {
            source_loc,
            policy_id,
            attribute_access,
        }
        .into()
    }

    pub(crate) fn unsafe_tag_access(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        entity_ty: Option<EntityLUB>,
        tag: Expr<Option<Type>>,
    ) -> Self {
        validation_errors::UnsafeTagAccess {
            source_loc,
            policy_id,
            entity_ty,
            tag,
        }
        .into()
    }

    pub(crate) fn no_tags_allowed(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        entity_ty: Option<EntityType>,
    ) -> Self {
        validation_errors::NoTagsAllowed {
            source_loc,
            policy_id,
            entity_ty,
        }
        .into()
    }

    pub(crate) fn undefined_extension(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        name: String,
    ) -> Self {
        validation_errors::UndefinedFunction {
            source_loc,
            policy_id,
            name,
        }
        .into()
    }

    pub(crate) fn wrong_number_args(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        expected: usize,
        actual: usize,
    ) -> Self {
        validation_errors::WrongNumberArguments {
            source_loc,
            policy_id,
            expected,
            actual,
        }
        .into()
    }

    pub(crate) fn function_argument_validation(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        msg: String,
    ) -> Self {
        validation_errors::FunctionArgumentValidation {
            source_loc,
            policy_id,
            msg,
        }
        .into()
    }

    pub(crate) fn empty_set_forbidden(source_loc: Option<Loc>, policy_id: PolicyID) -> Self {
        validation_errors::EmptySetForbidden {
            source_loc,
            policy_id,
        }
        .into()
    }

    pub(crate) fn non_lit_ext_constructor(source_loc: Option<Loc>, policy_id: PolicyID) -> Self {
        validation_errors::NonLitExtConstructor {
            source_loc,
            policy_id,
        }
        .into()
    }

    pub(crate) fn hierarchy_not_respected(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        in_lhs: Option<EntityType>,
        in_rhs: Option<EntityType>,
    ) -> Self {
        validation_errors::HierarchyNotRespected {
            source_loc,
            policy_id,
            in_lhs,
            in_rhs,
        }
        .into()
    }

    pub(crate) fn internal_invariant_violation(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
    ) -> Self {
        validation_errors::InternalInvariantViolation {
            source_loc,
            policy_id,
        }
        .into()
    }
}

/// Represents the different kinds of validation warnings and information
/// specific to that warning.
#[derive(Debug, Clone, PartialEq, Diagnostic, Error, Eq, Hash)]
pub enum ValidationWarning {
    /// A string contains mixed scripts. Different scripts can contain visually similar characters which may be confused for each other.
    #[diagnostic(transparent)]
    #[error(transparent)]
    MixedScriptString(#[from] validation_warnings::MixedScriptString),
    /// A string contains BIDI control characters. These can be used to create crafted pieces of code that obfuscate true control flow.
    #[diagnostic(transparent)]
    #[error(transparent)]
    BidiCharsInString(#[from] validation_warnings::BidiCharsInString),
    /// An id contains BIDI control characters. These can be used to create crafted pieces of code that obfuscate true control flow.
    #[diagnostic(transparent)]
    #[error(transparent)]
    BidiCharsInIdentifier(#[from] validation_warnings::BidiCharsInIdentifier),
    /// An id contains mixed scripts. This can cause characters to be confused for each other.
    #[diagnostic(transparent)]
    #[error(transparent)]
    MixedScriptIdentifier(#[from] validation_warnings::MixedScriptIdentifier),
    /// An id contains characters that fall outside of the General Security Profile for Identifiers. We recommend adhering to this if possible. See Unicode® Technical Standard #39 for more info.
    #[diagnostic(transparent)]
    #[error(transparent)]
    ConfusableIdentifier(#[from] validation_warnings::ConfusableIdentifier),
    /// The typechecker found that a policy condition will always evaluate to false.
    #[diagnostic(transparent)]
    #[error(transparent)]
    ImpossiblePolicy(#[from] validation_warnings::ImpossiblePolicy),
}

impl ValidationWarning {
    pub(crate) fn mixed_script_string(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        string: impl Into<String>,
    ) -> Self {
        validation_warnings::MixedScriptString {
            source_loc,
            policy_id,
            string: string.into(),
        }
        .into()
    }

    pub(crate) fn bidi_chars_strings(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        string: impl Into<String>,
    ) -> Self {
        validation_warnings::BidiCharsInString {
            source_loc,
            policy_id,
            string: string.into(),
        }
        .into()
    }

    pub(crate) fn mixed_script_identifier(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        id: impl Into<String>,
    ) -> Self {
        validation_warnings::MixedScriptIdentifier {
            source_loc,
            policy_id,
            id: id.into(),
        }
        .into()
    }

    pub(crate) fn bidi_chars_identifier(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        id: impl Into<String>,
    ) -> Self {
        validation_warnings::BidiCharsInIdentifier {
            source_loc,
            policy_id,
            id: id.into(),
        }
        .into()
    }

    pub(crate) fn confusable_identifier(
        source_loc: Option<Loc>,
        policy_id: PolicyID,
        id: impl Into<String>,
        confusable_character: char,
    ) -> Self {
        validation_warnings::ConfusableIdentifier {
            source_loc,
            policy_id,
            id: id.into(),
            confusable_character,
        }
        .into()
    }

    pub(crate) fn impossible_policy(source_loc: Option<Loc>, policy_id: PolicyID) -> Self {
        validation_warnings::ImpossiblePolicy {
            source_loc,
            policy_id,
        }
        .into()
    }
}
