use std::fs::{create_dir_all, File};
use std::io::{BufReader, Write};
use std::path::PathBuf;

use anyhow::Context;
use nostr::Keys;
use nostr_sdk::SecretKey;
use serde::{Deserialize, Serialize};

/// Nip47 Nostr Wallet Connect keys.
/// spec: https://github.com/nostr-protocol/nips/blob/master/47.md
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Nip47Keys {
    pub server_key: SecretKey,
    pub user_key: SecretKey,
    #[serde(default)]
    pub sent_info: bool,
}

impl Nip47Keys {
    /// Generates new Nip47Keys with fresh server and user keys.
    fn new() -> Result<Self, anyhow::Error> {
        let server_key = Keys::generate();
        let user_key = Keys::generate();

        Ok(Nip47Keys {
            server_key: nostr_sdk::SecretKey::from_slice(server_key.secret_key()?.as_ref())
                .context("Failed to convert to nostr_sdk::SecretKey")?,
            user_key: nostr_sdk::SecretKey::from_slice(user_key.secret_key()?.as_ref())
                .context("Failed to convert to nostr_sdk::SecretKey")?,
            sent_info: false,
        })
    }

    /// Returns a new `Keys` instance using the server key.
    pub fn server_keys(&self) -> Keys {
        Keys::new(self.server_key.clone().into())
    }

    /// Returns a new `Keys` instance using the user key.
    pub fn user_keys(&self) -> Keys {
        Keys::new(self.user_key.clone().into())
    }

    /// Retrieves keys from a file or generates and writes new keys if the file
    /// does not exist.
    pub fn load_or_generate_keys(keys_file: PathBuf) -> Result<Nip47Keys, anyhow::Error> {
        match File::open(&keys_file) {
            Ok(file) => {
                let reader = BufReader::new(file);
                serde_json::from_reader(reader).context("Could not parse JSON")
            }
            Err(_) => {
                let keys = Nip47Keys::new()?;
                Ok(Nip47Keys::save_keys(keys, &keys_file)?)
            }
        }
    }

    /// Serializes the keys and writes them to the specified path.
    fn save_keys(keys: Nip47Keys, path: &PathBuf) -> Result<Nip47Keys, anyhow::Error> {
        let json_str = serde_json::to_string(&keys).context("Could not serialize data")?;

        if let Some(parent) = path.parent() {
            create_dir_all(parent).context("Could not create directory")?;
        }

        let mut file = File::create(path).context("Could not create file")?;
        file.write_all(json_str.as_bytes())
            .context("Could not write to file")?;

        Ok(keys)
    }

    /// Writes the keys to the specified path.
    pub fn write_keys(&self, path: &PathBuf) -> Result<Nip47Keys, anyhow::Error> {
        let keys = self.clone();
        Nip47Keys::save_keys(keys, path)
    }
}
