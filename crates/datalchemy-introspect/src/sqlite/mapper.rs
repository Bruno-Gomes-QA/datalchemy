//! Map raw SQLite PRAGMA results to datalchemy-core types.

use std::collections::BTreeMap;

use sqlx::SqlitePool;

use datalchemy_core::types::ColumnType;
use datalchemy_core::{
    CheckConstraint, Column, Constraint, FkAction, FkMatchType, ForeignKey, Index, PrimaryKey,
    Table, TableKind, UniqueConstraint,
};

use super::queries::{RawColumn, RawForeignKey, RawIndex};

/// Build a [`Table`] from raw column info, including PK and CHECK extraction.
pub fn map_table(name: &str, raw_columns: Vec<RawColumn>) -> Table {
    let pk_columns: Vec<String> = raw_columns
        .iter()
        .filter(|c| c.pk)
        .map(|c| c.name.clone())
        .collect();

    let columns: Vec<Column> = raw_columns
        .iter()
        .enumerate()
        .map(|(i, raw)| Column {
            ordinal_position: (i + 1) as i16,
            name: raw.name.clone(),
            column_type: map_column_type(&raw.col_type),
            is_nullable: !raw.notnull && !raw.pk,
            default: raw.dflt_value.clone(),
            identity: None,
            generated: None,
            comment: None,
        })
        .collect();

    let mut constraints = Vec::new();
    if !pk_columns.is_empty() {
        constraints.push(Constraint::PrimaryKey(PrimaryKey {
            name: None,
            columns: pk_columns,
        }));
    }

    Table {
        name: name.to_string(),
        kind: TableKind::Table,
        comment: None,
        columns,
        constraints,
        indexes: Vec::new(),
    }
}

/// Map a SQLite type string to a [`ColumnType`].
fn map_column_type(raw: &str) -> ColumnType {
    let upper = raw.to_uppercase();

    let (base, max_len, precision, scale) = if let Some(paren_start) = upper.find('(') {
        let base = upper[..paren_start].trim().to_string();
        let inner = upper[paren_start + 1..].trim_end_matches(')').to_string();
        if let Some(comma_idx) = inner.find(',') {
            let p = inner[..comma_idx].trim().parse::<i32>().ok();
            let s = inner[comma_idx + 1..].trim().parse::<i32>().ok();
            (base, None, p, s)
        } else {
            let len = inner.trim().parse::<i32>().ok();
            if base.contains("CHAR") || base.contains("TEXT") || base.contains("CLOB") {
                (base, len, None, None)
            } else {
                (base, None, len, None)
            }
        }
    } else {
        (upper.clone(), None, None, None)
    };

    ColumnType {
        data_type: raw.to_string(),
        udt_schema: "main".to_string(),
        udt_name: base.to_lowercase(),
        character_max_length: max_len,
        numeric_precision: precision,
        numeric_scale: scale,
        collation: None,
    }
}

/// Map raw foreign key rows to [`ForeignKey`] values.
///
/// SQLite groups FK columns by `id`; multi-column FKs share the same `id`.
pub fn map_foreign_keys(raw: Vec<RawForeignKey>) -> Vec<ForeignKey> {
    let mut grouped: BTreeMap<i64, Vec<&RawForeignKey>> = BTreeMap::new();
    for fk in &raw {
        grouped.entry(fk.id).or_default().push(fk);
    }

    grouped
        .into_values()
        .map(|mut parts| {
            parts.sort_by_key(|p| p.seq);
            let first = &parts[0];
            ForeignKey {
                name: None,
                columns: parts.iter().map(|p| p.from.clone()).collect(),
                referenced_schema: "main".to_string(),
                referenced_table: first.table.clone(),
                referenced_columns: parts.iter().map(|p| p.to.clone()).collect(),
                on_update: map_fk_action(&first.on_update),
                on_delete: map_fk_action(&first.on_delete),
                match_type: map_fk_match(&first.r#match),
                is_deferrable: false,
                initially_deferred: false,
            }
        })
        .collect()
}

fn map_fk_action(raw: &str) -> FkAction {
    match raw.to_uppercase().as_str() {
        "NO ACTION" => FkAction::NoAction,
        "RESTRICT" => FkAction::Restrict,
        "CASCADE" => FkAction::Cascade,
        "SET NULL" => FkAction::SetNull,
        "SET DEFAULT" => FkAction::SetDefault,
        _ => FkAction::NoAction,
    }
}

fn map_fk_match(raw: &str) -> FkMatchType {
    match raw.to_uppercase().as_str() {
        "FULL" => FkMatchType::Full,
        "PARTIAL" => FkMatchType::Partial,
        "SIMPLE" | "NONE" | "" => FkMatchType::Simple,
        _ => FkMatchType::Simple,
    }
}

/// Map raw indexes to [`Index`] values, fetching column details.
pub async fn map_indexes(raw: Vec<RawIndex>, pool: &SqlitePool, table: &str) -> Vec<Index> {
    let mut indexes = Vec::new();
    for idx in raw {
        // Skip auto-created indexes for PRIMARY KEY / UNIQUE constraints
        if idx.origin == "pk" || idx.origin == "u" {
            continue;
        }

        let cols = super::queries::list_index_columns(pool, &idx.name)
            .await
            .unwrap_or_default();
        let col_list = cols.join(", ");
        let definition = format!(
            "CREATE{}INDEX \"{}\" ON \"{}\" ({})",
            if idx.unique { " UNIQUE " } else { " " },
            idx.name,
            table,
            col_list
        );

        indexes.push(Index {
            name: idx.name,
            is_unique: idx.unique,
            is_primary: idx.origin == "pk",
            is_valid: true,
            method: "btree".to_string(),
            definition,
        });
    }
    indexes
}
