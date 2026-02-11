use crate::dialect::Dialect;

/// A marker trait for types that can be used as SQL parameters.
///
/// This trait is used as a bound on the parameter type in SQL fragments.
/// It ensures type safety when building SQL queries with parameters.
pub trait SQLParam: Clone + core::fmt::Debug {
    /// The SQL dialect for this parameter type
    const DIALECT: Dialect;
}

// Implement SQLParam for common types
// impl<T: SQLParam> SQLParam for Option<T> {}
// impl<T: SQLParam> SQLParam for Vec<T> {}
// impl<T: SQLParam> SQLParam for Box<[T]> {}
// impl<T: SQLParam> SQLParam for Rc<T> {}
// impl<T: SQLParam> SQLParam for Arc<T> {}
// impl<T: SQLParam> SQLParam for RefCell<T> {}
// impl<'a, T: SQLParam> SQLParam for Cow<'a, T> {}
// impl<T: SQLParam> SQLParam for &[T] {}
// impl<T: SQLParam> SQLParam for &T {}
// impl<const N: usize, T: SQLParam> SQLParam for [T; N] {}
