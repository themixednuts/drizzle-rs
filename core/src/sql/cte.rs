//! Common table expression (CTE) support.
//!
//! CTE construction is exposed through builder APIs (e.g. `SelectBuilder::as_cte()`),
//! which return a typed CTE view with field access. This module is intentionally
//! light to avoid duplicating builder logic while still documenting the surface.
