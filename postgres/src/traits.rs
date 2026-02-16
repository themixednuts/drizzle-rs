mod column;
mod table;
mod value;

#[cfg(not(feature = "std"))]
use crate::prelude::*;
pub use column::*;
use core::any::Any;
use drizzle_core::error::DrizzleError;
pub use table::*;
pub use value::*;

/// Trait for PostgreSQL native enum types that can be used as dyn objects
#[allow(clippy::wrong_self_convention)]
pub trait PostgresEnum: Send + Sync + Any {
    /// Get the enum type name for PostgreSQL
    fn enum_type_name(&self) -> &'static str;

    fn as_enum(&self) -> &dyn PostgresEnum;

    /// Get the string representation of this enum variant
    fn variant_name(&self) -> &'static str;

    /// Clone this enum as a boxed trait object
    fn into_boxed(&self) -> Box<dyn PostgresEnum>;

    /// Try to create this enum from a string value
    fn try_from_str(value: &str) -> Result<Self, DrizzleError>
    where
        Self: Sized;
}

impl core::fmt::Debug for &dyn PostgresEnum {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PostgresSQLEnum")
            .field("type", &self.enum_type_name())
            .field("variant", &self.variant_name())
            .finish()
    }
}

impl PartialEq for &dyn PostgresEnum {
    fn eq(&self, other: &Self) -> bool {
        self.enum_type_name() == other.enum_type_name()
            && self.variant_name() == other.variant_name()
    }
}

impl core::fmt::Debug for Box<dyn PostgresEnum> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PostgresSQLEnum")
            .field("type", &self.enum_type_name())
            .field("variant", &self.variant_name())
            .finish()
    }
}

impl Clone for Box<dyn PostgresEnum> {
    fn clone(&self) -> Self {
        self.into_boxed()
    }
}

impl PartialEq for Box<dyn PostgresEnum> {
    fn eq(&self, other: &Self) -> bool {
        self.enum_type_name() == other.enum_type_name()
            && self.variant_name() == other.variant_name()
    }
}
