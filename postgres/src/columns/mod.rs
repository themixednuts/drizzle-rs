//! PostgreSQL column type builders with documentation.
//!
//! These builders provide documented `const fn` methods that configure column properties.
//! The macro generates code using these builders, giving users hover documentation.

mod bigint;
mod boolean;
mod bytea;
mod char;
mod date;
mod double_precision;
mod integer;
mod jsonb;
mod numeric;
mod real;
mod serial;
mod smallint;
mod text;
mod time;
mod timestamp;
mod uuid;
mod varchar;

pub use bigint::*;
pub use boolean::*;
pub use bytea::*;
pub use char::*;
pub use date::*;
pub use double_precision::*;
pub use integer::*;
pub use jsonb::*;
pub use numeric::*;
pub use real::*;
pub use serial::*;
pub use smallint::*;
pub use text::*;
pub use time::*;
pub use timestamp::*;
pub use uuid::*;
pub use varchar::*;
