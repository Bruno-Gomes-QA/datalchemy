use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::fmt::time::UtcTime;

use super::{RegistryError, RegistryResult};

pub fn init_run_logging(path: &Path) -> RegistryResult<()> {
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let file = Arc::new(Mutex::new(file));

    let make_writer = BoxMakeWriter::new(move || SharedWriter {
        file: Arc::clone(&file),
    });

    let layer = tracing_subscriber::fmt::layer()
        .json()
        .with_timer(UtcTime::rfc_3339())
        .with_writer(make_writer);

    tracing_subscriber::registry()
        .with(layer)
        .try_init()
        .map_err(|err| RegistryError::Logging(err.to_string()))?;

    Ok(())
}

struct SharedWriter {
    file: Arc<Mutex<std::fs::File>>,
}

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut file = self.file.lock().map_err(|_| {
            io::Error::new(io::ErrorKind::Other, "failed to lock log file")
        })?;
        file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut file = self.file.lock().map_err(|_| {
            io::Error::new(io::ErrorKind::Other, "failed to lock log file")
        })?;
        file.flush()
    }
}
