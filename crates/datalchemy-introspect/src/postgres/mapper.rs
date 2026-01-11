use datalchemy_core::{
    CheckConstraint, Column, ColumnType, Constraint, EnumType, ForeignKey, GeneratedExpression,
    GeneratedKind, Index, PrimaryKey, Table, TableKind, UniqueConstraint,
};

use crate::options::IntrospectOptions;
use crate::postgres::utils::{
    fk_action_from_code, fk_match_from_code, identity_from_text, relkind_to_table_kind,
};

use super::queries::{
    RawCheckConstraint, RawColumn, RawEnumType, RawForeignKey, RawIndex, RawPrimaryKey, RawTable,
    RawUniqueConstraint,
};

pub fn filter_schemas(raw: Vec<String>, opts: &IntrospectOptions) -> Vec<String> {
    raw.into_iter()
        .filter(|schema| {
            let is_system = schema.starts_with("pg_") || schema == "information_schema";
            match &opts.schemas {
                Some(list) => list.iter().any(|item| item == schema),
                None => opts.include_system_schemas || !is_system,
            }
        })
        .collect()
}

pub fn map_tables(raw: Vec<RawTable>, opts: &IntrospectOptions) -> Vec<Table> {
    raw.into_iter()
        .filter_map(|table| {
            let kind = relkind_to_table_kind(table.relkind);
            if !table_kind_enabled(&kind, opts) {
                return None;
            }

            let comment = if opts.include_comments {
                table.comment
            } else {
                None
            };

            Some(Table {
                name: table.name,
                kind,
                comment,
                columns: Vec::new(),
                constraints: Vec::new(),
                indexes: Vec::new(),
            })
        })
        .collect()
}

fn table_kind_enabled(kind: &TableKind, opts: &IntrospectOptions) -> bool {
    match kind {
        TableKind::View => opts.include_views,
        TableKind::MaterializedView => opts.include_materialized_views,
        TableKind::ForeignTable => opts.include_foreign_tables,
        _ => true,
    }
}

pub fn map_columns(raw: Vec<RawColumn>, opts: &IntrospectOptions) -> Vec<Column> {
    raw.into_iter()
        .map(|col| Column {
            ordinal_position: col.ordinal_position,
            name: col.name,
            column_type: ColumnType {
                data_type: col.data_type,
                udt_schema: col.udt_schema,
                udt_name: col.udt_name,
                character_max_length: col.character_max_length,
                numeric_precision: col.numeric_precision,
                numeric_scale: col.numeric_scale,
                collation: col.collation,
            },
            is_nullable: col.is_nullable,
            default: col.default,
            identity: identity_from_text(col.identity_generation),
            generated: if col.is_generated {
                Some(GeneratedExpression {
                    kind: GeneratedKind::Stored,
                    expression: col.generation_expression,
                })
            } else {
                None
            },
            comment: if opts.include_comments {
                col.comment
            } else {
                None
            },
        })
        .collect()
}

pub fn map_primary_key(raw: Option<RawPrimaryKey>) -> Option<PrimaryKey> {
    raw.map(|pk| PrimaryKey {
        name: Some(pk.name),
        columns: pk.columns,
    })
}

pub fn map_unique_constraints(raw: Vec<RawUniqueConstraint>) -> Vec<UniqueConstraint> {
    raw.into_iter()
        .map(|uc| UniqueConstraint {
            name: Some(uc.name),
            columns: uc.columns,
            is_deferrable: uc.is_deferrable,
            initially_deferred: uc.initially_deferred,
        })
        .collect()
}

pub fn map_check_constraints(raw: Vec<RawCheckConstraint>) -> Vec<CheckConstraint> {
    raw.into_iter()
        .map(|cc| CheckConstraint {
            name: Some(cc.name),
            expression: cc.expression,
        })
        .collect()
}

pub fn map_foreign_keys(raw: Vec<RawForeignKey>) -> Vec<ForeignKey> {
    raw.into_iter()
        .map(|fk| ForeignKey {
            name: Some(fk.name),
            columns: fk.columns,
            referenced_schema: fk.referenced_schema,
            referenced_table: fk.referenced_table,
            referenced_columns: fk.referenced_columns,
            on_update: fk_action_from_code(fk.on_update_code),
            on_delete: fk_action_from_code(fk.on_delete_code),
            match_type: fk_match_from_code(fk.match_type_code),
            is_deferrable: fk.is_deferrable,
            initially_deferred: fk.initially_deferred,
        })
        .collect()
}

pub fn map_indexes(raw: Vec<RawIndex>) -> Vec<Index> {
    raw.into_iter()
        .map(|idx| Index {
            name: idx.name,
            is_unique: idx.is_unique,
            is_primary: idx.is_primary,
            is_valid: idx.is_valid,
            method: idx.method,
            definition: idx.definition,
        })
        .collect()
}

pub fn map_enums(raw: Vec<RawEnumType>, opts: &IntrospectOptions) -> Vec<EnumType> {
    let allowed_schemas =
        filter_schemas(raw.iter().map(|item| item.schema.clone()).collect(), opts);

    raw.into_iter()
        .filter(|en| allowed_schemas.iter().any(|schema| schema == &en.schema))
        .map(|en| EnumType {
            schema: en.schema,
            name: en.name,
            labels: en.labels,
        })
        .collect()
}

pub fn sort_constraints(constraints: &mut Vec<Constraint>) {
    constraints.sort_by(|left, right| constraint_key(left).cmp(&constraint_key(right)));
}

fn constraint_key(constraint: &Constraint) -> (u8, String, String) {
    match constraint {
        Constraint::PrimaryKey(pk) => {
            (0, pk.name.clone().unwrap_or_default(), pk.columns.join("|"))
        }
        Constraint::Unique(unique) => (
            1,
            unique.name.clone().unwrap_or_default(),
            unique.columns.join("|"),
        ),
        Constraint::Check(check) => (
            2,
            check.name.clone().unwrap_or_default(),
            check.expression.clone(),
        ),
        Constraint::ForeignKey(fk) => {
            (3, fk.name.clone().unwrap_or_default(), fk.columns.join("|"))
        }
    }
}
