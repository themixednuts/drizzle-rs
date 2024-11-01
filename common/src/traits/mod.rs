mod column;
mod table;

pub trait Comparable<Rhs: ?Sized> {}
pub use column::*;
pub use table::*;

impl Comparable<i64> for i64 {}
impl Comparable<&i64> for i64 {}

impl Comparable<f64> for i64 {}
impl Comparable<&f64> for i64 {}

impl Comparable<f64> for f64 {}
impl Comparable<&f64> for f64 {}

impl Comparable<Self> for String {}
impl<'a> Comparable<String> for &'a String {}
impl<'a> Comparable<Self> for &'a String {}

impl Comparable<Self> for Vec<u8> {}
impl<'a> Comparable<Self> for &'a Vec<u8> {}

impl<'a> Comparable<&'a [u8]> for Vec<u8> {}
impl<'a> Comparable<Self> for &'a [u8] {}

impl<'a> Comparable<&'a str> for String {}
impl<'a> Comparable<&'a str> for &'a str {}
