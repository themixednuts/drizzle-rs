pub trait Autoincrement: Clone + Copy + Default + Sync + Send {
    const AUTOINCREMENT: bool;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NotAutoincrement {}

impl Autoincrement for NotAutoincrement {
    const AUTOINCREMENT: bool = false;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct IsAutoincrement {}

impl Autoincrement for IsAutoincrement {
    const AUTOINCREMENT: bool = true;
}
