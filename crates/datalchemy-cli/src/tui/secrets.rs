use std::io::{Read, Write};
use std::path::Path;

use age::secrecy::SecretString;
use age::{Decryptor, Encryptor};
use serde::{Deserialize, Serialize};

use crate::CliError;
use crate::tui::utils::set_private_permissions;
use crate::workspace::write_bytes_atomic;

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultMeta {
    pub status: String,
    pub created_at: Option<String>,
}

pub fn load_env_file(path: &Path) -> Result<std::collections::BTreeMap<String, String>, CliError> {
    let content = std::fs::read_to_string(path)?;
    let mut values = std::collections::BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();
        if key.is_empty() {
            continue;
        }
        values.insert(key.to_string(), value.to_string());
    }
    Ok(values)
}

pub fn encrypt_to_file(path: &Path, passphrase: &str, plaintext: &str) -> Result<(), CliError> {
    let secret = SecretString::from(passphrase.to_string());
    let encryptor = Encryptor::with_user_passphrase(secret);
    let mut output = Vec::new();
    {
        let mut writer = encryptor
            .wrap_output(&mut output)
            .map_err(|err| CliError::Crypto(err.to_string()))?;
        writer.write_all(plaintext.as_bytes())?;
        writer
            .finish()
            .map_err(|err| CliError::Crypto(err.to_string()))?;
    }
    write_bytes_atomic(path, &output)?;
    set_private_permissions(path)?;
    Ok(())
}

pub fn decrypt_from_file(path: &Path, passphrase: &str) -> Result<String, CliError> {
    let data = std::fs::read(path)?;
    let decryptor = Decryptor::new(&data[..]).map_err(|err| CliError::Crypto(err.to_string()))?;
    let identity = age::scrypt::Identity::new(SecretString::from(passphrase.to_string()));
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|err| CliError::Crypto(err.to_string()))?;
    let mut out = String::new();
    reader.read_to_string(&mut out)?;
    Ok(out)
}
