use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use datalchemy_core::Table;

use crate::generators::GeneratedValue;

/// Write a table as CSV with deterministic column ordering.
pub fn write_table_csv(
    path: &Path,
    table: &Table,
    rows: &[HashMap<String, GeneratedValue>],
) -> Result<u64, csv::Error> {
    let writer = BufWriter::new(File::create(path).map_err(csv::Error::from)?);
    let counting = CountingWriter::new(writer);
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(counting);

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
    let counting = writer.into_inner().map_err(|err| err.into_error())?;
    Ok(counting.bytes_written())
}

struct CountingWriter<W: Write> {
    inner: W,
    bytes: u64,
}

impl<W: Write> CountingWriter<W> {
    fn new(inner: W) -> Self {
        Self { inner, bytes: 0 }
    }

    fn bytes_written(&self) -> u64 {
        self.bytes
    }
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let size = self.inner.write(buf)?;
        self.bytes = self.bytes.saturating_add(size as u64);
        Ok(size)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
