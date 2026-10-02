//! Ready-made generators to pass to `SeedConfig::generator`.
//!
//! Every function returns a [`Generator`]. The [`GeneratorExt`] methods
//! combine them: [`nullable`](GeneratorExt::nullable) mixes in `NULL`s and
//! [`map`](GeneratorExt::map) post-processes values.
//!
//! # Examples
//!
//! ```rust
//! use drizzle_seed::generators::{self, GeneratorExt};
//! use drizzle_seed::{Generator, SeedValue};
//! # use rand::SeedableRng;
//! # let mut rng = rand::rngs::StdRng::seed_from_u64(1);
//!
//! let age = generators::int(18..=90);
//! let plan = generators::one_of(["free", "pro", "team"]);
//! let bio = generators::words(5..=20).nullable(0.3);
//! let slug = generators::from_fn(|_rng, index| format!("post-{}", index + 1));
//!
//! assert!(matches!(age.generate(&mut rng, 0, "INTEGER"), SeedValue::Integer(18..=90)));
//! assert!(matches!(plan.generate(&mut rng, 0, "TEXT"), SeedValue::Text(_)));
//! assert!(matches!(bio.generate(&mut rng, 0, "TEXT"), SeedValue::Text(_) | SeedValue::Null));
//! assert_eq!(slug.generate(&mut rng, 4, "TEXT"), SeedValue::Text("post-5".into()));
//! ```
//!
//! With a schema, set them per column:
//!
//! ```text
//! SeedConfig::postgres(&schema)
//!     .generator(&schema.users.age, generators::int(18..=90))
//!     .generator(&schema.users.plan, generators::weighted([("free", 8), ("pro", 2)]))
//!     .generator(&schema.users.nickname, generators::first_name().nullable(0.5))
//!     .generate();
//! ```

use crate::generator::{Generator, GeneratorKind, RngCore, SeedValue, string};
use core::range::RangeInclusive;
use rand::Rng;

/// A range accepted by the generator constructors: `a..b` or `a..=b`,
/// written with range syntax or as a `core::range` value.
///
/// Generators store it as a [`core::range::RangeInclusive`], which is
/// `Copy` and has public `start` and `last` fields.
pub trait IntoRange<T>: sealed::Sealed<T> {
    /// The same values as an inclusive range, or `None` when it is empty.
    fn into_range(self) -> Option<RangeInclusive<T>>;
}

mod sealed {
    pub trait Sealed<T> {}
}

macro_rules! integer_ranges {
    ($($ty:ty),*) => {$(
        impl sealed::Sealed<$ty> for core::ops::Range<$ty> {}
        impl IntoRange<$ty> for core::ops::Range<$ty> {
            fn into_range(self) -> Option<RangeInclusive<$ty>> {
                (self.start < self.end).then(|| RangeInclusive {
                    start: self.start,
                    last: self.end - 1,
                })
            }
        }
        impl sealed::Sealed<$ty> for core::range::Range<$ty> {}
        impl IntoRange<$ty> for core::range::Range<$ty> {
            fn into_range(self) -> Option<RangeInclusive<$ty>> {
                core::ops::Range::from(self).into_range()
            }
        }
        impl sealed::Sealed<$ty> for core::ops::RangeInclusive<$ty> {}
        impl IntoRange<$ty> for core::ops::RangeInclusive<$ty> {
            fn into_range(self) -> Option<RangeInclusive<$ty>> {
                (!self.is_empty()).then(|| self.into())
            }
        }
        impl sealed::Sealed<$ty> for RangeInclusive<$ty> {}
        impl IntoRange<$ty> for RangeInclusive<$ty> {
            fn into_range(self) -> Option<RangeInclusive<$ty>> {
                (self.start <= self.last).then_some(self)
            }
        }
    )*};
}

integer_ranges!(i64, usize);

impl sealed::Sealed<f64> for core::ops::RangeInclusive<f64> {}
impl IntoRange<f64> for core::ops::RangeInclusive<f64> {
    fn into_range(self) -> Option<RangeInclusive<f64>> {
        RangeInclusive::from(self).into_range()
    }
}
impl sealed::Sealed<f64> for RangeInclusive<f64> {}
impl IntoRange<f64> for RangeInclusive<f64> {
    fn into_range(self) -> Option<RangeInclusive<f64>> {
        (self.start.is_finite() && self.last.is_finite() && self.start <= self.last).then_some(self)
    }
}

/// Combinators available on every [`Generator`].
pub trait GeneratorExt: Generator + Sized {
    /// Returns `NULL` with the given probability (0.0 to 1.0), otherwise a
    /// value from `self`.
    ///
    /// # Panics
    ///
    /// Panics if `probability` is not between 0.0 and 1.0.
    fn nullable(self, probability: f64) -> Nullable<Self> {
        assert!(
            (0.0..=1.0).contains(&probability),
            "nullable probability must be between 0.0 and 1.0, got {probability}"
        );
        Nullable {
            inner: self,
            probability,
        }
    }

    /// Transforms each generated value with `f`.
    fn map<F, V>(self, f: F) -> Map<Self, F>
    where
        F: Fn(SeedValue) -> V + Send + Sync,
        V: Into<SeedValue>,
    {
        Map { inner: self, f }
    }
}

impl<G: Generator> GeneratorExt for G {}

/// Random integers from a range. Created by [`int`].
#[derive(Debug, Clone, Copy)]
pub struct Int {
    range: RangeInclusive<i64>,
}

/// Random integers in `range`, for example `int(18..=90)` or `int(0..10)`.
///
/// # Panics
///
/// Panics if the range is empty.
#[must_use]
pub fn int(range: impl IntoRange<i64>) -> Int {
    Int {
        range: range.into_range().expect("int range is empty"),
    }
}

impl Generator for Int {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        SeedValue::Integer(rng.random_range(self.range.start..=self.range.last))
    }

    fn name(&self) -> &'static str {
        "int"
    }
}

/// Random floating-point numbers from a range. Created by [`float`].
#[derive(Debug, Clone, Copy)]
pub struct Float {
    range: RangeInclusive<f64>,
    decimals: Option<u32>,
}

/// Random numbers in `range`, for example `float(0.0..=100.0)`.
///
/// # Panics
///
/// Panics if the range is empty or not finite.
#[must_use]
pub fn float(range: impl IntoRange<f64>) -> Float {
    Float {
        range: range
            .into_range()
            .expect("float range must be finite and non-empty"),
        decimals: None,
    }
}

impl Float {
    /// Rounds each value to `decimals` digits after the point, for example
    /// prices with `.decimals(2)`.
    #[must_use]
    pub const fn decimals(mut self, decimals: u32) -> Self {
        self.decimals = Some(decimals);
        self
    }
}

impl Generator for Float {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let value = rng.random_range(self.range.start..=self.range.last);
        let value = match self.decimals {
            Some(decimals) => {
                let factor = 10f64.powi(i32::try_from(decimals).unwrap_or(i32::MAX));
                ((value * factor).round() / factor).clamp(self.range.start, self.range.last)
            }
            None => value,
        };
        SeedValue::Float(value)
    }

    fn name(&self) -> &'static str {
        "float"
    }
}

/// Random booleans. Created by [`boolean`].
#[derive(Debug, Clone, Copy)]
pub struct Boolean {
    probability_true: f64,
}

/// `true` with the given probability (0.0 to 1.0), otherwise `false`.
///
/// # Panics
///
/// Panics if `probability_true` is not between 0.0 and 1.0.
#[must_use]
pub fn boolean(probability_true: f64) -> Boolean {
    assert!(
        (0.0..=1.0).contains(&probability_true),
        "boolean probability must be between 0.0 and 1.0, got {probability_true}"
    );
    Boolean { probability_true }
}

impl Generator for Boolean {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        SeedValue::Bool(rng.random_bool(self.probability_true))
    }

    fn name(&self) -> &'static str {
        "boolean"
    }
}

/// Picks one of a fixed set of values. Created by [`one_of`].
#[derive(Debug, Clone)]
pub struct OneOf {
    values: Vec<SeedValue>,
}

/// Picks one of `values`, each equally likely, for example
/// `one_of(["free", "pro"])`.
///
/// # Panics
///
/// Panics if `values` is empty.
#[must_use]
pub fn one_of<T: Into<SeedValue>>(values: impl IntoIterator<Item = T>) -> OneOf {
    let values: Vec<SeedValue> = values.into_iter().map(Into::into).collect();
    assert!(!values.is_empty(), "one_of needs at least one value");
    OneOf { values }
}

impl Generator for OneOf {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        self.values[rng.random_range(0..self.values.len())].clone()
    }

    fn name(&self) -> &'static str {
        "one_of"
    }
}

/// Picks values in proportion to their weights. Created by [`weighted`].
#[derive(Debug, Clone)]
pub struct Weighted {
    values: Vec<SeedValue>,
    cumulative: Vec<u64>,
}

/// Picks one of `values` in proportion to its weight, for example
/// `weighted([("free", 8), ("pro", 2)])` gives `"free"` 80% of the time.
///
/// # Panics
///
/// Panics if `values` is empty or every weight is zero.
#[must_use]
pub fn weighted<T: Into<SeedValue>>(values: impl IntoIterator<Item = (T, u32)>) -> Weighted {
    let mut total = 0u64;
    let (values, cumulative) = values
        .into_iter()
        .map(|(value, weight)| {
            total += u64::from(weight);
            (value.into(), total)
        })
        .unzip();
    assert!(total > 0, "weighted needs at least one non-zero weight");
    Weighted { values, cumulative }
}

impl Generator for Weighted {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        let total = *self.cumulative.last().unwrap_or(&1);
        let pick = rng.random_range(0..total);
        let index = self.cumulative.partition_point(|&end| end <= pick);
        self.values[index].clone()
    }

    fn name(&self) -> &'static str {
        "weighted"
    }
}

/// The same value for every row. Created by [`constant`].
#[derive(Debug, Clone)]
pub struct Constant {
    value: SeedValue,
}

/// The same `value` for every row.
#[must_use]
pub fn constant(value: impl Into<SeedValue>) -> Constant {
    Constant {
        value: value.into(),
    }
}

impl Generator for Constant {
    fn generate(&self, _rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
        self.value.clone()
    }

    fn name(&self) -> &'static str {
        "constant"
    }
}

/// Counts up from a start value. Created by [`sequence`].
#[derive(Debug, Clone, Copy)]
pub struct Sequence {
    start: i64,
    step: i64,
}

/// `start`, `start + 1`, ... by row; change the increment with
/// [`step`](Sequence::step).
#[must_use]
pub const fn sequence(start: i64) -> Sequence {
    Sequence { start, step: 1 }
}

impl Sequence {
    /// Sets the increment between rows.
    #[must_use]
    pub const fn step(mut self, step: i64) -> Self {
        self.step = step;
        self
    }
}

impl Generator for Sequence {
    fn generate(&self, _rng: &mut dyn RngCore, index: usize, _sql_type: &str) -> SeedValue {
        let index = i64::try_from(index).unwrap_or(i64::MAX);
        SeedValue::Integer(self.start.saturating_add(index.saturating_mul(self.step)))
    }

    fn name(&self) -> &'static str {
        "sequence"
    }
}

/// Random lowercase letters. Created by [`text`].
#[derive(Debug, Clone, Copy)]
pub struct Text {
    length: RangeInclusive<usize>,
}

/// A string of random lowercase letters whose length is in `length`, for
/// example `text(5..=12)`.
///
/// # Panics
///
/// Panics if the range is empty.
#[must_use]
pub fn text(length: impl IntoRange<usize>) -> Text {
    Text {
        length: length.into_range().expect("text length range is empty"),
    }
}

impl Generator for Text {
    fn generate(&self, rng: &mut dyn RngCore, index: usize, sql_type: &str) -> SeedValue {
        string::TextGen {
            min_len: self.length.start,
            max_len: self.length.last,
        }
        .generate(rng, index, sql_type)
    }

    fn name(&self) -> &'static str {
        "text"
    }
}

/// Lorem ipsum words. Created by [`words`].
#[derive(Debug, Clone, Copy)]
pub struct Words {
    count: RangeInclusive<usize>,
}

/// Lorem ipsum: a number of words in `count`, separated by spaces, for
/// example `words(5..=20)`.
///
/// # Panics
///
/// Panics if the range is empty.
#[must_use]
pub fn words(count: impl IntoRange<usize>) -> Words {
    Words {
        count: count.into_range().expect("words count range is empty"),
    }
}

impl Generator for Words {
    fn generate(&self, rng: &mut dyn RngCore, index: usize, sql_type: &str) -> SeedValue {
        let words = rng.random_range(self.count.start..=self.count.last);
        string::LoremGen { words }.generate(rng, index, sql_type)
    }

    fn name(&self) -> &'static str {
        "words"
    }
}

/// A generator from a closure. Created by [`from_fn`].
pub struct FromFn<F> {
    f: F,
}

/// Uses a closure as a generator. It gets the RNG and the 0-based row
/// index, and returns anything that converts into a [`SeedValue`].
///
/// Draw randomness only from the given RNG, so the same seed gives the same
/// rows. Its methods come from [`Rng`], re-exported by this
/// crate.
#[must_use]
pub fn from_fn<F, V>(f: F) -> FromFn<F>
where
    F: Fn(&mut dyn RngCore, usize) -> V + Send + Sync,
    V: Into<SeedValue>,
{
    FromFn { f }
}

impl<F, V> Generator for FromFn<F>
where
    F: Fn(&mut dyn RngCore, usize) -> V + Send + Sync,
    V: Into<SeedValue>,
{
    fn generate(&self, rng: &mut dyn RngCore, index: usize, _sql_type: &str) -> SeedValue {
        (self.f)(rng, index).into()
    }

    fn name(&self) -> &'static str {
        "from_fn"
    }
}

/// Mixes `NULL` into another generator. Created by
/// [`GeneratorExt::nullable`].
pub struct Nullable<G> {
    inner: G,
    probability: f64,
}

impl<G: Generator> Generator for Nullable<G> {
    fn generate(&self, rng: &mut dyn RngCore, index: usize, sql_type: &str) -> SeedValue {
        if rng.random_bool(self.probability) {
            SeedValue::Null
        } else {
            self.inner.generate(rng, index, sql_type)
        }
    }

    fn name(&self) -> &'static str {
        self.inner.name()
    }
}

/// Transforms another generator's values. Created by [`GeneratorExt::map`].
pub struct Map<G, F> {
    inner: G,
    f: F,
}

impl<G, F, V> Generator for Map<G, F>
where
    G: Generator,
    F: Fn(SeedValue) -> V + Send + Sync,
    V: Into<SeedValue>,
{
    fn generate(&self, rng: &mut dyn RngCore, index: usize, sql_type: &str) -> SeedValue {
        (self.f)(self.inner.generate(rng, index, sql_type)).into()
    }

    fn name(&self) -> &'static str {
        self.inner.name()
    }
}

macro_rules! kind_constructors {
    ($($(#[$doc:meta])* $name:ident => $kind:ident,)*) => {$(
        $(#[$doc])*
        #[must_use]
        pub const fn $name() -> GeneratorKind {
            GeneratorKind::$kind
        }
    )*};
}

kind_constructors! {
    /// `first.last{row}@domain`; the row number keeps emails unique.
    email => Email,
    /// A first name.
    first_name => FirstName,
    /// A last name.
    last_name => LastName,
    /// A first and last name.
    full_name => FullName,
    /// A US-style phone number, `(555) 555-5555`.
    phone => Phone,
    /// A city name.
    city => City,
    /// A country name.
    country => Country,
    /// A street address, `123 Name Street`.
    address => Address,
    /// A company name, such as `Smith Inc`.
    company => Company,
    /// A job title.
    job_title => JobTitle,
    /// A random UUID v4 as text.
    uuid => Uuid,
    /// A small JSON object as text.
    json => Json,
    /// A date between 2000 and 2030 as `YYYY-MM-DD` (Unix milliseconds for
    /// an integer column).
    date => Date,
    /// A timestamp between 2000 and 2030 as `YYYY-MM-DD HH:MM:SS` (Unix
    /// milliseconds for an integer column).
    timestamp => Timestamp,
    /// A time of day, `HH:MM:SS`.
    time => Time,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn values(generator: &impl Generator, rows: usize) -> Vec<SeedValue> {
        let mut rng = StdRng::seed_from_u64(7);
        (0..rows)
            .map(|index| generator.generate(&mut rng, index, "TEXT"))
            .collect()
    }

    #[test]
    fn ranges_are_respected() {
        assert!(
            values(&int(-3..=3), 200)
                .iter()
                .all(|value| matches!(value, SeedValue::Integer(-3..=3)))
        );
        assert!(
            values(&float(1.0..=2.0).decimals(1), 200)
                .iter()
                .all(|value| {
                    matches!(value, SeedValue::Float(number) if (1.0..=2.0).contains(number)
                && (number * 10.0).fract() == 0.0)
                })
        );
        assert!(values(&text(2..=4), 200).iter().all(|value| {
            matches!(value, SeedValue::Text(text) if (2..=4).contains(&text.len()))
        }));
    }

    #[test]
    fn exclusive_inclusive_and_core_range_forms_agree() {
        let exclusive = values(&int(0..3), 200);
        assert_eq!(exclusive, values(&int(0..=2), 200));
        assert_eq!(
            exclusive,
            values(&int(core::range::Range { start: 0, end: 3 }), 200)
        );
        assert_eq!(
            exclusive,
            values(&int(core::range::RangeInclusive { start: 0, last: 2 }), 200)
        );
        assert_eq!(values(&text(3..4), 20), values(&text(3..=3), 20));
    }

    #[test]
    #[should_panic(expected = "int range is empty")]
    fn empty_ranges_are_rejected() {
        let _ = int(5..5);
    }

    #[test]
    fn choices_and_weights() {
        let picked = values(&one_of(["a", "b"]), 200);
        assert!(picked.contains(&SeedValue::from("a")) && picked.contains(&SeedValue::from("b")));

        let picked = values(&weighted([("never", 0), ("always", 3)]), 100);
        assert!(
            picked
                .iter()
                .all(|value| *value == SeedValue::from("always"))
        );
    }

    #[test]
    fn sequence_constant_and_closures() {
        assert_eq!(
            values(&sequence(10).step(5), 3),
            [10, 15, 20].map(SeedValue::from)
        );
        assert_eq!(
            values(&constant(true), 2),
            [true, true].map(SeedValue::from)
        );
        assert_eq!(
            values(&from_fn(|_rng, index| index as i64 * 2), 3),
            [0, 2, 4].map(SeedValue::from)
        );
    }

    #[test]
    fn combinators() {
        let mixed = values(&int(1..=1).nullable(0.5), 200);
        assert!(mixed.contains(&SeedValue::Null) && mixed.contains(&SeedValue::Integer(1)));
        assert!(
            values(&int(1..=1).nullable(0.0), 50)
                .iter()
                .all(|value| *value == SeedValue::Integer(1))
        );
        let mapped = values(
            &first_name().map(|value| match value {
                SeedValue::Text(name) => name.to_uppercase(),
                _ => unreachable!(),
            }),
            5,
        );
        assert!(mapped.iter().all(|value| matches!(
            value,
            SeedValue::Text(name) if name.chars().all(|c| !c.is_lowercase())
        )));
    }
}
