//! PostgreSQL View DDL types
//!
//! This module provides two complementary types:
//! - [`ViewDef`] - A const-friendly definition type for compile-time schema definitions
//! - [`View`] - A runtime type for serde serialization/deserialization

#[cfg(feature = "std")]
use std::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;

#[cfg(feature = "serde")]
use crate::serde_helpers::{cow_from_string, cow_option_from_string};

// =============================================================================
// ViewWithOption Types
// =============================================================================

/// Const-friendly view WITH options definition
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ViewWithOptionDef {
    /// CHECK OPTION ('local' | 'cascaded')
    pub check_option: Option<&'static str>,
    /// Security barrier flag
    pub security_barrier: bool,
    /// Security invoker flag
    pub security_invoker: bool,
    /// Fillfactor (for materialized views)
    pub fillfactor: Option<i32>,
    /// Toast tuple target (for materialized views)
    pub toast_tuple_target: Option<i32>,
    /// Parallel workers (for materialized views)
    pub parallel_workers: Option<i32>,
    /// Autovacuum enabled (for materialized views)
    pub autovacuum_enabled: Option<bool>,
    /// Vacuum index cleanup (for materialized views): 'auto' | 'on' | 'off'
    pub vacuum_index_cleanup: Option<&'static str>,
    /// Vacuum truncate (for materialized views)
    pub vacuum_truncate: Option<bool>,
    /// Autovacuum vacuum threshold (for materialized views)
    pub autovacuum_vacuum_threshold: Option<i32>,
    /// Autovacuum vacuum scale factor (for materialized views)
    pub autovacuum_vacuum_scale_factor: Option<i32>,
    /// Autovacuum vacuum cost delay (for materialized views)
    pub autovacuum_vacuum_cost_delay: Option<i32>,
    /// Autovacuum vacuum cost limit (for materialized views)
    pub autovacuum_vacuum_cost_limit: Option<i32>,
    /// Autovacuum freeze min age (for materialized views)
    pub autovacuum_freeze_min_age: Option<i64>,
    /// Autovacuum freeze max age (for materialized views)
    pub autovacuum_freeze_max_age: Option<i64>,
    /// Autovacuum freeze table age (for materialized views)
    pub autovacuum_freeze_table_age: Option<i64>,
    /// Autovacuum multixact freeze min age (for materialized views)
    pub autovacuum_multixact_freeze_min_age: Option<i64>,
    /// Autovacuum multixact freeze max age (for materialized views)
    pub autovacuum_multixact_freeze_max_age: Option<i64>,
    /// Autovacuum multixact freeze table age (for materialized views)
    pub autovacuum_multixact_freeze_table_age: Option<i64>,
    /// Log autovacuum min duration (for materialized views)
    pub log_autovacuum_min_duration: Option<i32>,
    /// User catalog table (for materialized views)
    pub user_catalog_table: Option<bool>,
}

impl ViewWithOptionDef {
    /// Create a new view WITH options definition
    #[must_use]
    pub const fn new() -> Self {
        Self {
            check_option: None,
            security_barrier: false,
            security_invoker: false,
            fillfactor: None,
            toast_tuple_target: None,
            parallel_workers: None,
            autovacuum_enabled: None,
            vacuum_index_cleanup: None,
            vacuum_truncate: None,
            autovacuum_vacuum_threshold: None,
            autovacuum_vacuum_scale_factor: None,
            autovacuum_vacuum_cost_delay: None,
            autovacuum_vacuum_cost_limit: None,
            autovacuum_freeze_min_age: None,
            autovacuum_freeze_max_age: None,
            autovacuum_freeze_table_age: None,
            autovacuum_multixact_freeze_min_age: None,
            autovacuum_multixact_freeze_max_age: None,
            autovacuum_multixact_freeze_table_age: None,
            log_autovacuum_min_duration: None,
            user_catalog_table: None,
        }
    }

    /// Set CHECK OPTION
    #[must_use]
    pub const fn check_option(self, option: &'static str) -> Self {
        Self {
            check_option: Some(option),
            ..self
        }
    }

    /// Set security barrier
    #[must_use]
    pub const fn security_barrier(self) -> Self {
        Self {
            security_barrier: true,
            ..self
        }
    }

    /// Set security invoker
    #[must_use]
    pub const fn security_invoker(self) -> Self {
        Self {
            security_invoker: true,
            ..self
        }
    }

    /// Set fillfactor (for materialized views)
    #[must_use]
    pub const fn fillfactor(self, value: i32) -> Self {
        Self {
            fillfactor: Some(value),
            ..self
        }
    }

    /// Set toast tuple target (for materialized views)
    #[must_use]
    pub const fn toast_tuple_target(self, value: i32) -> Self {
        Self {
            toast_tuple_target: Some(value),
            ..self
        }
    }

    /// Set parallel workers (for materialized views)
    #[must_use]
    pub const fn parallel_workers(self, value: i32) -> Self {
        Self {
            parallel_workers: Some(value),
            ..self
        }
    }

    /// Set autovacuum enabled (for materialized views)
    #[must_use]
    pub const fn autovacuum_enabled(self, value: bool) -> Self {
        Self {
            autovacuum_enabled: Some(value),
            ..self
        }
    }

    /// Set vacuum index cleanup (for materialized views): "auto", "on", or "off"
    #[must_use]
    pub const fn vacuum_index_cleanup(self, value: &'static str) -> Self {
        Self {
            vacuum_index_cleanup: Some(value),
            ..self
        }
    }

    /// Set vacuum truncate (for materialized views)
    #[must_use]
    pub const fn vacuum_truncate(self, value: bool) -> Self {
        Self {
            vacuum_truncate: Some(value),
            ..self
        }
    }

    /// Set autovacuum vacuum threshold (for materialized views)
    #[must_use]
    pub const fn autovacuum_vacuum_threshold(self, value: i32) -> Self {
        Self {
            autovacuum_vacuum_threshold: Some(value),
            ..self
        }
    }

    /// Set autovacuum vacuum scale factor (for materialized views)
    #[must_use]
    pub const fn autovacuum_vacuum_scale_factor(self, value: i32) -> Self {
        Self {
            autovacuum_vacuum_scale_factor: Some(value),
            ..self
        }
    }

    /// Set autovacuum vacuum cost delay (for materialized views)
    #[must_use]
    pub const fn autovacuum_vacuum_cost_delay(self, value: i32) -> Self {
        Self {
            autovacuum_vacuum_cost_delay: Some(value),
            ..self
        }
    }

    /// Set autovacuum vacuum cost limit (for materialized views)
    #[must_use]
    pub const fn autovacuum_vacuum_cost_limit(self, value: i32) -> Self {
        Self {
            autovacuum_vacuum_cost_limit: Some(value),
            ..self
        }
    }

    /// Set autovacuum freeze min age (for materialized views)
    #[must_use]
    pub const fn autovacuum_freeze_min_age(self, value: i64) -> Self {
        Self {
            autovacuum_freeze_min_age: Some(value),
            ..self
        }
    }

    /// Set autovacuum freeze max age (for materialized views)
    #[must_use]
    pub const fn autovacuum_freeze_max_age(self, value: i64) -> Self {
        Self {
            autovacuum_freeze_max_age: Some(value),
            ..self
        }
    }

    /// Set autovacuum freeze table age (for materialized views)
    #[must_use]
    pub const fn autovacuum_freeze_table_age(self, value: i64) -> Self {
        Self {
            autovacuum_freeze_table_age: Some(value),
            ..self
        }
    }

    /// Set autovacuum multixact freeze min age (for materialized views)
    #[must_use]
    pub const fn autovacuum_multixact_freeze_min_age(self, value: i64) -> Self {
        Self {
            autovacuum_multixact_freeze_min_age: Some(value),
            ..self
        }
    }

    /// Set autovacuum multixact freeze max age (for materialized views)
    #[must_use]
    pub const fn autovacuum_multixact_freeze_max_age(self, value: i64) -> Self {
        Self {
            autovacuum_multixact_freeze_max_age: Some(value),
            ..self
        }
    }

    /// Set autovacuum multixact freeze table age (for materialized views)
    #[must_use]
    pub const fn autovacuum_multixact_freeze_table_age(self, value: i64) -> Self {
        Self {
            autovacuum_multixact_freeze_table_age: Some(value),
            ..self
        }
    }

    /// Set log autovacuum min duration (for materialized views)
    #[must_use]
    pub const fn log_autovacuum_min_duration(self, value: i32) -> Self {
        Self {
            log_autovacuum_min_duration: Some(value),
            ..self
        }
    }

    /// Set user catalog table (for materialized views)
    #[must_use]
    pub const fn user_catalog_table(self, value: bool) -> Self {
        Self {
            user_catalog_table: Some(value),
            ..self
        }
    }

    /// Convert to runtime type
    #[must_use]
    pub const fn into_view_with_option(self) -> ViewWithOption {
        ViewWithOption {
            check_option: match self.check_option {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
            security_barrier: if self.security_barrier {
                Some(true)
            } else {
                None
            },
            security_invoker: if self.security_invoker {
                Some(true)
            } else {
                None
            },
            fillfactor: self.fillfactor,
            toast_tuple_target: self.toast_tuple_target,
            parallel_workers: self.parallel_workers,
            autovacuum_enabled: self.autovacuum_enabled,
            vacuum_index_cleanup: match self.vacuum_index_cleanup {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
            vacuum_truncate: self.vacuum_truncate,
            autovacuum_vacuum_threshold: self.autovacuum_vacuum_threshold,
            autovacuum_vacuum_scale_factor: self.autovacuum_vacuum_scale_factor,
            autovacuum_vacuum_cost_delay: self.autovacuum_vacuum_cost_delay,
            autovacuum_vacuum_cost_limit: self.autovacuum_vacuum_cost_limit,
            autovacuum_freeze_min_age: self.autovacuum_freeze_min_age,
            autovacuum_freeze_max_age: self.autovacuum_freeze_max_age,
            autovacuum_freeze_table_age: self.autovacuum_freeze_table_age,
            autovacuum_multixact_freeze_min_age: self.autovacuum_multixact_freeze_min_age,
            autovacuum_multixact_freeze_max_age: self.autovacuum_multixact_freeze_max_age,
            autovacuum_multixact_freeze_table_age: self.autovacuum_multixact_freeze_table_age,
            log_autovacuum_min_duration: self.log_autovacuum_min_duration,
            user_catalog_table: self.user_catalog_table,
        }
    }
}

impl Default for ViewWithOptionDef {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime view WITH options entity
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ViewWithOption {
    /// CHECK OPTION ('local' | 'cascaded')
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub check_option: Option<Cow<'static, str>>,

    /// Security barrier flag
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub security_barrier: Option<bool>,

    /// Security invoker flag
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub security_invoker: Option<bool>,

    /// Fillfactor (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub fillfactor: Option<i32>,

    /// Toast tuple target (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub toast_tuple_target: Option<i32>,

    /// Parallel workers (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub parallel_workers: Option<i32>,

    /// Autovacuum enabled (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub autovacuum_enabled: Option<bool>,

    /// Vacuum index cleanup (for materialized views): 'auto' | 'on' | 'off'
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub vacuum_index_cleanup: Option<Cow<'static, str>>,

    /// Vacuum truncate (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub vacuum_truncate: Option<bool>,

    /// Autovacuum vacuum threshold (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub autovacuum_vacuum_threshold: Option<i32>,

    /// Autovacuum vacuum scale factor (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub autovacuum_vacuum_scale_factor: Option<i32>,

    /// Autovacuum vacuum cost delay (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub autovacuum_vacuum_cost_delay: Option<i32>,

    /// Autovacuum vacuum cost limit (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub autovacuum_vacuum_cost_limit: Option<i32>,

    /// Autovacuum freeze min age (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub autovacuum_freeze_min_age: Option<i64>,

    /// Autovacuum freeze max age (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub autovacuum_freeze_max_age: Option<i64>,

    /// Autovacuum freeze table age (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub autovacuum_freeze_table_age: Option<i64>,

    /// Autovacuum multixact freeze min age (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub autovacuum_multixact_freeze_min_age: Option<i64>,

    /// Autovacuum multixact freeze max age (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub autovacuum_multixact_freeze_max_age: Option<i64>,

    /// Autovacuum multixact freeze table age (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub autovacuum_multixact_freeze_table_age: Option<i64>,

    /// Log autovacuum min duration (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub log_autovacuum_min_duration: Option<i32>,

    /// User catalog table (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub user_catalog_table: Option<bool>,
}

impl Default for ViewWithOption {
    fn default() -> Self {
        ViewWithOptionDef::new().into_view_with_option()
    }
}

impl From<ViewWithOptionDef> for ViewWithOption {
    fn from(def: ViewWithOptionDef) -> Self {
        def.into_view_with_option()
    }
}

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// Const-friendly view definition
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ViewDef {
    /// Schema name
    pub schema: &'static str,
    /// View name
    pub name: &'static str,
    /// View definition (AS SELECT ...)
    pub definition: Option<&'static str>,
    /// Is this a materialized view?
    pub materialized: bool,
    /// WITH options
    pub with: Option<ViewWithOptionDef>,
    /// Whether this is an existing view (not managed by drizzle)
    pub is_existing: bool,
    /// WITH NO DATA (for materialized views)
    pub with_no_data: bool,
    /// USING clause (for materialized views)
    pub using: Option<&'static str>,
    /// Tablespace (for materialized views)
    pub tablespace: Option<&'static str>,
}

impl ViewDef {
    /// Create a new view definition
    #[must_use]
    pub const fn new(schema: &'static str, name: &'static str) -> Self {
        Self {
            schema,
            name,
            definition: None,
            materialized: false,
            with: None,
            is_existing: false,
            with_no_data: false,
            using: None,
            tablespace: None,
        }
    }

    /// Set the view definition
    #[must_use]
    pub const fn definition(self, sql: &'static str) -> Self {
        Self {
            definition: Some(sql),
            ..self
        }
    }

    /// Mark as materialized view
    #[must_use]
    pub const fn materialized(self) -> Self {
        Self {
            materialized: true,
            ..self
        }
    }

    /// Set WITH options
    #[must_use]
    pub const fn with_options(self, options: ViewWithOptionDef) -> Self {
        Self {
            with: Some(options),
            ..self
        }
    }

    /// Mark as existing (not managed by drizzle)
    #[must_use]
    pub const fn existing(self) -> Self {
        Self {
            is_existing: true,
            ..self
        }
    }

    /// Set WITH NO DATA
    #[must_use]
    pub const fn with_no_data(self) -> Self {
        Self {
            with_no_data: true,
            ..self
        }
    }

    /// Set USING clause
    #[must_use]
    pub const fn using(self, clause: &'static str) -> Self {
        Self {
            using: Some(clause),
            ..self
        }
    }

    /// Set tablespace
    #[must_use]
    pub const fn tablespace(self, space: &'static str) -> Self {
        Self {
            tablespace: Some(space),
            ..self
        }
    }

    /// Convert to runtime [`View`] type
    ///
    /// Note: This method cannot be const because it needs to convert nested Option types
    /// (with options) which require runtime method calls.
    #[must_use]
    pub fn into_view(self) -> View {
        View {
            schema: Cow::Borrowed(self.schema),
            name: Cow::Borrowed(self.name),
            definition: match self.definition {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
            materialized: self.materialized,
            with: self.with.map(|w| w.into_view_with_option()),
            is_existing: self.is_existing,
            with_no_data: if self.with_no_data { Some(true) } else { None },
            using: match self.using {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
            tablespace: match self.tablespace {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
        }
    }
}

impl Default for ViewDef {
    fn default() -> Self {
        Self::new("public", "")
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime view entity
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct View {
    /// Schema name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub schema: Cow<'static, str>,

    /// View name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,

    /// View definition (AS SELECT ...)
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub definition: Option<Cow<'static, str>>,

    /// Is this a materialized view?
    #[cfg_attr(feature = "serde", serde(default))]
    pub materialized: bool,

    /// WITH options
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", rename = "with")
    )]
    pub with: Option<ViewWithOption>,

    /// Whether this is an existing view (not managed by drizzle)
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_existing: bool,

    /// WITH NO DATA (for materialized views)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub with_no_data: Option<bool>,

    /// USING clause (for materialized views)
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub using: Option<Cow<'static, str>>,

    /// Tablespace (for materialized views)
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub tablespace: Option<Cow<'static, str>>,
}

impl View {
    /// Create a new view
    #[must_use]
    pub fn new(schema: impl Into<Cow<'static, str>>, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            schema: schema.into(),
            name: name.into(),
            definition: None,
            materialized: false,
            with: None,
            is_existing: false,
            with_no_data: None,
            using: None,
            tablespace: None,
        }
    }

    /// Get the schema name
    #[inline]
    #[must_use]
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// Get the view name
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Default for View {
    fn default() -> Self {
        Self::new("public", "")
    }
}

impl From<ViewDef> for View {
    fn from(def: ViewDef) -> Self {
        def.into_view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_view_def() {
        const VIEW: ViewDef = ViewDef::new("public", "active_users")
            .definition("SELECT * FROM users WHERE active = 1");

        assert_eq!(VIEW.name, "active_users");
        assert_eq!(VIEW.schema, "public");
    }

    #[test]
    fn test_materialized_view_def() {
        const MAT_VIEW: ViewDef = ViewDef::new("public", "user_stats")
            .materialized()
            .with_no_data();

        assert!(MAT_VIEW.materialized);
    }

    #[test]
    fn test_view_def_to_view() {
        const DEF: ViewDef = ViewDef::new("public", "view").definition("SELECT 1");
        let view = DEF.into_view();
        assert_eq!(view.name(), "view");
        assert_eq!(view.schema(), "public");
    }

    #[test]
    fn test_view_with_option_def_builders() {
        // Test const builder methods for materialized view options
        const OPTIONS: ViewWithOptionDef = ViewWithOptionDef::new()
            .fillfactor(80)
            .parallel_workers(4)
            .autovacuum_enabled(true)
            .vacuum_index_cleanup("auto")
            .vacuum_truncate(false)
            .autovacuum_vacuum_threshold(100)
            .autovacuum_vacuum_scale_factor(20)
            .autovacuum_vacuum_cost_delay(10)
            .autovacuum_vacuum_cost_limit(200)
            .autovacuum_freeze_min_age(50_000_000)
            .autovacuum_freeze_max_age(200_000_000)
            .autovacuum_freeze_table_age(150_000_000)
            .autovacuum_multixact_freeze_min_age(5_000_000)
            .autovacuum_multixact_freeze_max_age(400_000_000)
            .autovacuum_multixact_freeze_table_age(150_000_000)
            .log_autovacuum_min_duration(1000)
            .user_catalog_table(false)
            .toast_tuple_target(128);

        assert_eq!(OPTIONS.fillfactor, Some(80));
        assert_eq!(OPTIONS.parallel_workers, Some(4));
        assert_eq!(OPTIONS.autovacuum_enabled, Some(true));
        assert_eq!(OPTIONS.vacuum_index_cleanup, Some("auto"));
        assert_eq!(OPTIONS.vacuum_truncate, Some(false));
        assert_eq!(OPTIONS.autovacuum_vacuum_threshold, Some(100));
        assert_eq!(OPTIONS.autovacuum_vacuum_scale_factor, Some(20));
        assert_eq!(OPTIONS.autovacuum_vacuum_cost_delay, Some(10));
        assert_eq!(OPTIONS.autovacuum_vacuum_cost_limit, Some(200));
        assert_eq!(OPTIONS.autovacuum_freeze_min_age, Some(50_000_000));
        assert_eq!(OPTIONS.autovacuum_freeze_max_age, Some(200_000_000));
        assert_eq!(OPTIONS.autovacuum_freeze_table_age, Some(150_000_000));
        assert_eq!(OPTIONS.autovacuum_multixact_freeze_min_age, Some(5_000_000));
        assert_eq!(
            OPTIONS.autovacuum_multixact_freeze_max_age,
            Some(400_000_000)
        );
        assert_eq!(
            OPTIONS.autovacuum_multixact_freeze_table_age,
            Some(150_000_000)
        );
        assert_eq!(OPTIONS.log_autovacuum_min_duration, Some(1000));
        assert_eq!(OPTIONS.user_catalog_table, Some(false));
        assert_eq!(OPTIONS.toast_tuple_target, Some(128));
    }

    #[test]
    fn test_view_with_option_def_to_runtime() {
        const OPTIONS: ViewWithOptionDef = ViewWithOptionDef::new()
            .fillfactor(90)
            .security_barrier()
            .security_invoker()
            .check_option("cascaded");

        let runtime = OPTIONS.into_view_with_option();
        assert_eq!(runtime.fillfactor, Some(90));
        assert_eq!(runtime.security_barrier, Some(true));
        assert_eq!(runtime.security_invoker, Some(true));
        assert_eq!(runtime.check_option.as_deref(), Some("cascaded"));
    }

    #[test]
    fn test_materialized_view_with_all_options() {
        const MAT_VIEW: ViewDef = ViewDef::new("analytics", "monthly_sales")
            .materialized()
            .with_no_data()
            .using("btree")
            .tablespace("fast_ssd")
            .with_options(ViewWithOptionDef::new().fillfactor(90).parallel_workers(2))
            .definition("SELECT * FROM sales WHERE date > now() - interval '30 days'");

        assert!(MAT_VIEW.materialized);
        assert!(MAT_VIEW.with_no_data);
        assert_eq!(MAT_VIEW.using, Some("btree"));
        assert_eq!(MAT_VIEW.tablespace, Some("fast_ssd"));
        assert!(MAT_VIEW.with.is_some());

        let options = MAT_VIEW.with.unwrap();
        assert_eq!(options.fillfactor, Some(90));
        assert_eq!(options.parallel_workers, Some(2));
    }
}
