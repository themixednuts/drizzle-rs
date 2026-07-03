use core::any::Any;

use crate::{SQLParam, SQLSchemaType, SQLTable, TableRef, ToSQL};

/// Compile-time PostgreSQL row-level security policy metadata.
///
/// Implementing this trait automatically provides [`SQLPolicyInfo`] via a
/// blanket implementation.
pub trait DrizzlePolicy: Send + Sync + 'static {
    /// Policy name.
    const POLICY_NAME: &'static str;

    /// AS clause (`PERMISSIVE` or `RESTRICTIVE`).
    const AS_CLAUSE: Option<&'static str> = None;

    /// FOR clause (`ALL`, `SELECT`, `INSERT`, `UPDATE`, or `DELETE`).
    const FOR_CLAUSE: Option<&'static str> = None;

    /// TO roles.
    const TO: &'static [&'static str] = &[];

    /// USING expression.
    const USING: Option<&'static str> = None;

    /// WITH CHECK expression.
    const WITH_CHECK: Option<&'static str> = None;

    /// The table this policy belongs to.
    fn table_ref() -> &'static TableRef;
}

/// Blanket: any [`DrizzlePolicy`] automatically satisfies [`SQLPolicyInfo`].
impl<T: DrizzlePolicy> SQLPolicyInfo for T {
    fn table(&self) -> &'static TableRef {
        T::table_ref()
    }

    fn name(&self) -> &'static str {
        T::POLICY_NAME
    }

    fn as_clause(&self) -> Option<&'static str> {
        T::AS_CLAUSE
    }

    fn for_clause(&self) -> Option<&'static str> {
        T::FOR_CLAUSE
    }

    fn to(&self) -> &'static [&'static str] {
        T::TO
    }

    fn using(&self) -> Option<&'static str> {
        T::USING
    }

    fn with_check(&self) -> Option<&'static str> {
        T::WITH_CHECK
    }
}

/// Runtime view of policy metadata for schema derivation.
pub trait SQLPolicyInfo: Any + Send + Sync {
    fn table(&self) -> &'static TableRef;
    fn name(&self) -> &'static str;
    fn as_clause(&self) -> Option<&'static str>;
    fn for_clause(&self) -> Option<&'static str>;
    fn to(&self) -> &'static [&'static str];
    fn using(&self) -> Option<&'static str>;
    fn with_check(&self) -> Option<&'static str>;
}

impl core::fmt::Debug for dyn SQLPolicyInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SQLPolicyInfo")
            .field("name", &self.name())
            .field("table", &self.table().name)
            .field("as_clause", &self.as_clause())
            .field("for_clause", &self.for_clause())
            .field("to", &self.to())
            .finish()
    }
}

/// Trait for types that represent PostgreSQL policies.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a SQL policy for this dialect",
    label = "ensure this type was derived with #[PostgresPolicy]"
)]
pub trait SQLPolicy<'a, Type: SQLSchemaType, Value: SQLParam + 'a>:
    SQLPolicyInfo + ToSQL<'a, Value>
{
    /// The table type this policy is associated with.
    type Table: SQLTable<'a, Type, Value>;
}
