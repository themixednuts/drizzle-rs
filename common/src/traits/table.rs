pub trait Table {
    type Schema;
    fn name(&self) -> &str;
    fn schema(&self) -> Self::Schema;
}
