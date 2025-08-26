use std::{borrow::Cow, cell::RefCell, rc::Rc, sync::Arc};

/// A marker trait for types that can be used as SQL parameters.
///
/// This trait is used as a bound on the parameter type in SQL fragments.
/// It ensures type safety when building SQL queries with parameters.
pub trait SQLParam: Clone + std::fmt::Debug {}

// Implement SQLParam for common types
impl SQLParam for char {}
impl SQLParam for String {}
impl SQLParam for &str {}
impl SQLParam for i8 {}
impl SQLParam for i16 {}
impl SQLParam for i32 {}
impl SQLParam for i64 {}
impl SQLParam for isize {}
impl SQLParam for u8 {}
impl SQLParam for u16 {}
impl SQLParam for u32 {}
impl SQLParam for u64 {}
impl SQLParam for usize {}
impl SQLParam for f32 {}
impl SQLParam for f64 {}
impl SQLParam for bool {}
impl<T: SQLParam> SQLParam for Option<T> {}
impl<T: SQLParam> SQLParam for Vec<T> {}
impl<T: SQLParam> SQLParam for Box<[T]> {}
impl<T: SQLParam> SQLParam for Rc<T> {}
impl<T: SQLParam> SQLParam for Arc<T> {}
impl<T: SQLParam> SQLParam for RefCell<T> {}
impl<'a, T: SQLParam> SQLParam for Cow<'a, T> {}
impl<T: SQLParam> SQLParam for &[T] {}
impl<T: SQLParam> SQLParam for &T {}
impl<const N: usize, T: SQLParam> SQLParam for [T; N] {}
