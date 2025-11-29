//! Schema diff types and logic for PostgreSQL

use super::{Column, Enum, ForeignKey, Index, PostgresSnapshot, Sequence, Table};
use std::collections::HashMap;

/// Complete schema diff between two PostgreSQL snapshots
#[derive(Debug, Clone, Default)]
pub struct SchemaDiff {
    pub tables: TablesDiff,
    pub enums: EnumsDiff,
    pub sequences: SequencesDiff,
}

impl SchemaDiff {
    pub fn has_changes(&self) -> bool {
        self.tables.has_changes() || self.enums.has_changes() || self.sequences.has_changes()
    }

    pub fn is_empty(&self) -> bool {
        !self.has_changes()
    }
}

/// Table-level diff
#[derive(Debug, Clone, Default)]
pub struct TablesDiff {
    pub created: Vec<Table>,
    pub deleted: Vec<String>,
    pub altered: Vec<AlteredTable>,
}

impl TablesDiff {
    pub fn has_changes(&self) -> bool {
        !self.created.is_empty() || !self.deleted.is_empty() || !self.altered.is_empty()
    }
}

/// Enum-level diff
#[derive(Debug, Clone, Default)]
pub struct EnumsDiff {
    pub created: Vec<Enum>,
    pub deleted: Vec<String>,
    pub altered: Vec<AlteredEnum>,
}

impl EnumsDiff {
    pub fn has_changes(&self) -> bool {
        !self.created.is_empty() || !self.deleted.is_empty() || !self.altered.is_empty()
    }
}

/// Sequence-level diff
#[derive(Debug, Clone, Default)]
pub struct SequencesDiff {
    pub created: Vec<Sequence>,
    pub deleted: Vec<String>,
    pub altered: Vec<(Sequence, Sequence)>,
}

impl SequencesDiff {
    pub fn has_changes(&self) -> bool {
        !self.created.is_empty() || !self.deleted.is_empty() || !self.altered.is_empty()
    }
}

/// An altered table
#[derive(Debug, Clone)]
pub struct AlteredTable {
    pub name: String,
    pub schema: String,
    pub columns: ColumnsDiff,
    pub indexes: IndexesDiff,
    pub foreign_keys: ForeignKeysDiff,
}

impl AlteredTable {
    pub fn has_changes(&self) -> bool {
        self.columns.has_changes() || self.indexes.has_changes() || self.foreign_keys.has_changes()
    }
}

/// An altered enum
#[derive(Debug, Clone)]
pub struct AlteredEnum {
    pub name: String,
    pub schema: String,
    pub values_added: Vec<String>,
    pub values_removed: Vec<String>,
}

/// Column-level diff
#[derive(Debug, Clone, Default)]
pub struct ColumnsDiff {
    pub added: Vec<Column>,
    pub deleted: Vec<String>,
    pub altered: Vec<AlteredColumn>,
}

impl ColumnsDiff {
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.deleted.is_empty() || !self.altered.is_empty()
    }
}

/// An altered column
#[derive(Debug, Clone)]
pub struct AlteredColumn {
    pub name: String,
    pub old: Column,
    pub new: Column,
}

/// Index-level diff
#[derive(Debug, Clone, Default)]
pub struct IndexesDiff {
    pub added: Vec<Index>,
    pub deleted: Vec<String>,
    pub altered: Vec<(Index, Index)>,
}

impl IndexesDiff {
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.deleted.is_empty() || !self.altered.is_empty()
    }
}

/// Foreign key diff
#[derive(Debug, Clone, Default)]
pub struct ForeignKeysDiff {
    pub added: Vec<ForeignKey>,
    pub deleted: Vec<String>,
    pub altered: Vec<(ForeignKey, ForeignKey)>,
}

impl ForeignKeysDiff {
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.deleted.is_empty() || !self.altered.is_empty()
    }
}

/// Compare two PostgreSQL snapshots
pub fn diff_snapshots(prev: &PostgresSnapshot, cur: &PostgresSnapshot) -> SchemaDiff {
    SchemaDiff {
        tables: diff_tables(&prev.tables, &cur.tables),
        enums: diff_enums(&prev.enums, &cur.enums),
        sequences: diff_sequences(&prev.sequences, &cur.sequences),
    }
}

fn diff_tables(prev: &HashMap<String, Table>, cur: &HashMap<String, Table>) -> TablesDiff {
    let mut diff = TablesDiff::default();

    for (name, table) in cur {
        if !prev.contains_key(name) {
            diff.created.push(table.clone());
        }
    }

    for name in prev.keys() {
        if !cur.contains_key(name) {
            diff.deleted.push(name.clone());
        }
    }

    for (name, cur_table) in cur {
        if let Some(prev_table) = prev.get(name) {
            let altered = diff_table(prev_table, cur_table);
            if altered.has_changes() {
                diff.altered.push(altered);
            }
        }
    }

    diff
}

fn diff_table(prev: &Table, cur: &Table) -> AlteredTable {
    AlteredTable {
        name: cur.name.clone(),
        schema: cur.schema.clone(),
        columns: diff_columns(&prev.columns, &cur.columns),
        indexes: diff_indexes(&prev.indexes, &cur.indexes),
        foreign_keys: diff_foreign_keys(&prev.foreign_keys, &cur.foreign_keys),
    }
}

fn diff_columns(prev: &HashMap<String, Column>, cur: &HashMap<String, Column>) -> ColumnsDiff {
    let mut diff = ColumnsDiff::default();

    for (name, column) in cur {
        if !prev.contains_key(name) {
            diff.added.push(column.clone());
        }
    }

    for name in prev.keys() {
        if !cur.contains_key(name) {
            diff.deleted.push(name.clone());
        }
    }

    for (name, cur_col) in cur {
        if let Some(prev_col) = prev.get(name) {
            if prev_col != cur_col {
                diff.altered.push(AlteredColumn {
                    name: name.clone(),
                    old: prev_col.clone(),
                    new: cur_col.clone(),
                });
            }
        }
    }

    diff
}

fn diff_indexes(prev: &HashMap<String, Index>, cur: &HashMap<String, Index>) -> IndexesDiff {
    let mut diff = IndexesDiff::default();

    for (name, index) in cur {
        if !prev.contains_key(name) {
            diff.added.push(index.clone());
        }
    }

    for name in prev.keys() {
        if !cur.contains_key(name) {
            diff.deleted.push(name.clone());
        }
    }

    for (name, cur_idx) in cur {
        if let Some(prev_idx) = prev.get(name) {
            if prev_idx != cur_idx {
                diff.altered.push((prev_idx.clone(), cur_idx.clone()));
            }
        }
    }

    diff
}

fn diff_foreign_keys(
    prev: &HashMap<String, ForeignKey>,
    cur: &HashMap<String, ForeignKey>,
) -> ForeignKeysDiff {
    let mut diff = ForeignKeysDiff::default();

    for (name, fk) in cur {
        if !prev.contains_key(name) {
            diff.added.push(fk.clone());
        }
    }

    for name in prev.keys() {
        if !cur.contains_key(name) {
            diff.deleted.push(name.clone());
        }
    }

    for (name, cur_fk) in cur {
        if let Some(prev_fk) = prev.get(name) {
            if prev_fk != cur_fk {
                diff.altered.push((prev_fk.clone(), cur_fk.clone()));
            }
        }
    }

    diff
}

fn diff_enums(prev: &HashMap<String, Enum>, cur: &HashMap<String, Enum>) -> EnumsDiff {
    let mut diff = EnumsDiff::default();

    for (name, e) in cur {
        if !prev.contains_key(name) {
            diff.created.push(e.clone());
        }
    }

    for name in prev.keys() {
        if !cur.contains_key(name) {
            diff.deleted.push(name.clone());
        }
    }

    for (name, cur_enum) in cur {
        if let Some(prev_enum) = prev.get(name) {
            let added: Vec<_> = cur_enum
                .values
                .iter()
                .filter(|v| !prev_enum.values.contains(v))
                .cloned()
                .collect();
            let removed: Vec<_> = prev_enum
                .values
                .iter()
                .filter(|v| !cur_enum.values.contains(v))
                .cloned()
                .collect();

            if !added.is_empty() || !removed.is_empty() {
                diff.altered.push(AlteredEnum {
                    name: cur_enum.name.clone(),
                    schema: cur_enum.schema.clone(),
                    values_added: added,
                    values_removed: removed,
                });
            }
        }
    }

    diff
}

fn diff_sequences(
    prev: &HashMap<String, Sequence>,
    cur: &HashMap<String, Sequence>,
) -> SequencesDiff {
    let mut diff = SequencesDiff::default();

    for (name, seq) in cur {
        if !prev.contains_key(name) {
            diff.created.push(seq.clone());
        }
    }

    for name in prev.keys() {
        if !cur.contains_key(name) {
            diff.deleted.push(name.clone());
        }
    }

    for (name, cur_seq) in cur {
        if let Some(prev_seq) = prev.get(name) {
            if prev_seq != cur_seq {
                diff.altered.push((prev_seq.clone(), cur_seq.clone()));
            }
        }
    }

    diff
}
