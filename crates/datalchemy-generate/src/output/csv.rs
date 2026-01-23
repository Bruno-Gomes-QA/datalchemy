use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use datalchemy_core::Table;

use crate::generators::GeneratedValue;

/// Write a table as CSV with deterministic column ordering.
pub fn write_table_csv(
    path: &Path,
    table: &Table,
    rows: &[HashMap<String, GeneratedValue>],
) -> Result<(), csv::Error> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(File::create(path).map_err(csv::Error::from)?);

    let mut columns = table.columns.clone();
    columns.sort_by_key(|col| col.ordinal_position);

    let header: Vec<String> = columns.iter().map(|col| col.name.clone()).collect();
    writer.write_record(&header)?;

    for row in rows {
        let record: Vec<String> = columns
            .iter()
            .map(|col| {
                row.get(&col.name.to_lowercase())
                    .map(|value| value.to_csv(col))
                    .unwrap_or_default()
            })
            .collect();
        writer.write_record(&record)?;
    }

    writer.flush()?;
    Ok(())
}
