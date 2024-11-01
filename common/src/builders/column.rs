#[allow(unused)]
#[derive(Default, Debug, Clone)]
pub struct ColumnBaseBuilder<DataType: Default, ColumnType: Default, DataMode: Default> {
    pub name: &'static str,
    pub mode: DataMode,
    pub data_type: DataType,
    pub column_type: ColumnType,
}
