use crate::{FkAction, FkMatchType, IdentityGeneration, TableKind};

/// Convert Postgres `relkind` code to a typed table kind.
pub fn relkind_to_table_kind(code: i8) -> TableKind {
    match code as u8 as char {
        'r' => TableKind::Table,
        'p' => TableKind::PartitionedTable,
        'v' => TableKind::View,
        'm' => TableKind::MaterializedView,
        'f' => TableKind::ForeignTable,
        other => TableKind::Other(other.to_string()),
    }
}

/// Convert FK action code to a descriptive enum.
pub fn fk_action_from_code(code: i8) -> FkAction {
    match code as u8 as char {
        'a' => FkAction::NoAction,
        'r' => FkAction::Restrict,
        'c' => FkAction::Cascade,
        'n' => FkAction::SetNull,
        'd' => FkAction::SetDefault,
        _ => FkAction::Unknown,
    }
}

/// Convert FK match type code to enum.
pub fn fk_match_from_code(code: i8) -> FkMatchType {
    match code as u8 as char {
        'f' => FkMatchType::Full,
        'p' => FkMatchType::Partial,
        's' => FkMatchType::Simple,
        _ => FkMatchType::Unknown,
    }
}

/// Map textual identity generation to the enum used in the model.
pub fn identity_from_text(identity: Option<String>) -> Option<IdentityGeneration> {
    identity.as_deref().and_then(|value| match value {
        "ALWAYS" => Some(IdentityGeneration::Always),
        "BY DEFAULT" => Some(IdentityGeneration::ByDefault),
        _ => None,
    })
}
