//! The actions that turn encryption on and off, and the migration that
//! converts a plain database into an encrypted one.
use super::*;

impl Store {
    /// Remove the write-ahead log and shared-memory files that belong to the
    /// database. They must not survive a change of the database file.
    fn remove_sidecar_files(&self) -> Result<()> {
        for suffix in ["-wal", "-shm"] {
            let sidecar = self.path.with_extension(format!(
                "{}{suffix}",
                self.path
                    .extension()
                    .map(|e| e.to_string_lossy().to_string())
                    .unwrap_or_default()
            ));
            if sidecar.exists() {
                let _ = fs::remove_file(sidecar);
            }
        }
        Ok(())
    }

    /// The encryption state of the live database.
    pub fn encryption_status(&self) -> Result<Value> {
        let state = encryption::read_state(&self.path)?;
        let header = if self.path.exists() {
            let bytes = fs::read(&self.path).unwrap_or_default();
            if bytes.len() >= 15 {
                String::from_utf8_lossy(&bytes[..15]).to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        Ok(json!({
            "protection":match state.protection {
                encryption::Protection::None => "none",
                encryption::Protection::Passphrase => "passphrase",
            },
            "description":state.protection.label(),
            "encrypted_on_disk":encryption::looks_encrypted(&self.path),
            "file_header":header,
            "state_file":encryption::state_path(&self.path).to_string_lossy(),
            "database_path":self.path.to_string_lossy(),
            "cipher":"sqlcipher-4",
            "note":"Encryption is off by default. When it is on, the database file is unreadable without the master key."
        }))
    }

    /// Turn encryption on for the live database.
    ///
    /// The key is derived from a passphrase with Argon2id. A verified plain
    /// backup is taken before anything changes.
    pub fn encryption_enable(&mut self, p: &Value, actor: &str) -> Result<Value> {
        let mode = optional(p, "mode").unwrap_or_else(|| "passphrase".into());
        if mode != "passphrase" {
            return Err(LifeError::Invalid("mode must be passphrase".into()));
        }
        let current = encryption::read_state(&self.path)?;
        if current.protection != encryption::Protection::None {
            return Err(LifeError::Conflict(
                "This database is already encrypted".into(),
            ));
        }
        let passphrase = if mode == "passphrase" {
            Some(require(p, "passphrase")?)
        } else {
            None
        };
        let secret = passphrase.ok_or_else(|| {
            LifeError::Invalid("A passphrase is required to encrypt the database".into())
        })?;
        let salt = encryption::new_salt()?;
        let params = encryption::Argon2Params::default();
        let key = encryption::derive_key(&secret, &salt, &params)?;

        // A verified backup of the plain database is taken before anything
        // changes, so the original can always be recovered.
        let backup = self.backup(&json!({}))?;
        let backup_path = PathBuf::from(backup["path"].as_str().unwrap_or(""));

        // The rows are exported into a new encrypted file, and the plain file
        // is replaced by it.
        let encrypted_path = self.path.with_extension("encrypted.sqlite");
        let _ = fs::remove_file(&encrypted_path);
        let target = Connection::open(&encrypted_path)?;
        target.execute_batch(&encryption::cipher_pragmas(&key))?;
        self.conn.execute_batch(&format!(
            "ATTACH DATABASE '{}' AS encrypted KEY \"x'{key}'\"; \
             SELECT sqlcipher_export('encrypted'); \
             DETACH DATABASE encrypted;",
            encrypted_path.to_string_lossy()
        ))?;
        drop(target);
        let check = encryption::open_encrypted(&encrypted_path, &key)?;
        let integrity: String = check.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if integrity != "ok" {
            let _ = fs::remove_file(&encrypted_path);
            return Err(LifeError::Conflict(
                "The encrypted copy failed its integrity check; the original is untouched".into(),
            ));
        }
        drop(check);
        // The old connection is closed before the file is replaced, and the
        // write-ahead log of the plain database is removed, so no stale page
        // survives into the encrypted file.
        self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
        self.conn = Connection::open_in_memory()?;
        self.remove_sidecar_files()?;
        fs::rename(&encrypted_path, &self.path)?;
        encryption::write_state(
            &self.path,
            &encryption::EncryptionState {
                protection: encryption::Protection::Passphrase,
                salt: Some(salt),
                params: Some(params),
            },
        )?;
        // The store now lives on the encrypted file.
        let reopened = encryption::open_encrypted(&self.path, &key)?;
        reopened.execute_batch(
            "PRAGMA cipher_page_size = 4096; PRAGMA cipher_hmac_algorithm = HMAC_SHA512; PRAGMA cipher_kdf_algorithm = PBKDF2_HMAC_SHA512; PRAGMA journal_mode=WAL;PRAGMA foreign_keys=ON;PRAGMA synchronous=NORMAL;",
        )?;
        self.swap_connection(reopened, Some(key.clone()))?;
        self.record(
            actor,
            "encryption.enable",
            None,
            &json!({"mode":mode,"pre_encryption_backup":backup_path.to_string_lossy()}),
        )?;
        Ok(json!({
            "encrypted":true,
            "mode":mode,
            "pre_encryption_backup":backup_path.to_string_lossy(),
            "state_file":encryption::state_path(&self.path).to_string_lossy(),
            "note":"The master key is never written to the database or to the repository."
        }))
    }

    /// Turn encryption off, after taking a verified encrypted backup.
    pub fn encryption_disable(&mut self, p: &Value, actor: &str) -> Result<Value> {
        let state = encryption::read_state(&self.path)?;
        if state.protection == encryption::Protection::None {
            return Err(LifeError::Conflict("This database is not encrypted".into()));
        }
        if !p.get("confirm").and_then(Value::as_bool).unwrap_or(false) {
            return Err(LifeError::Invalid(
                "Set confirm=true after reviewing that the database will become readable on disk"
                    .into(),
            ));
        }
        // The key is resolved so a missing or wrong key fails before the
        // database is touched, not halfway through the conversion.
        encryption::resolve_key_with(&state, self.key.as_deref())?;
        let backup = self.backup(&json!({}))?;
        let plain_path = self.path.with_extension("plain.sqlite");
        let _ = fs::remove_file(&plain_path);
        let target = Connection::open(&plain_path)?;
        self.conn.execute_batch(&format!(
            "ATTACH DATABASE '{}' AS plain KEY ''; \
             SELECT sqlcipher_export('plain'); \
             DETACH DATABASE plain;",
            plain_path.to_string_lossy()
        ))?;
        drop(target);
        let check = Connection::open(&plain_path)?;
        let integrity: String = check.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if integrity != "ok" {
            let _ = fs::remove_file(&plain_path);
            return Err(LifeError::Conflict(
                "The decrypted copy failed its integrity check; the encrypted original is untouched"
                    .into(),
            ));
        }
        drop(check);
        // The old connection is closed before the file is replaced, and the
        // write-ahead log of the encrypted database is removed.
        self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
        self.conn = Connection::open_in_memory()?;
        self.remove_sidecar_files()?;
        fs::rename(&plain_path, &self.path)?;
        encryption::clear_state(&self.path)?;
        let reopened = Connection::open(&self.path)?;
        reopened.execute_batch(
            "PRAGMA journal_mode=WAL;PRAGMA foreign_keys=ON;PRAGMA synchronous=NORMAL;",
        )?;
        self.swap_connection(reopened, None)?;
        self.record(
            actor,
            "encryption.disable",
            None,
            &json!({"pre_decryption_backup":backup["path"].clone()}),
        )?;
        Ok(json!({
            "encrypted":false,
            "pre_decryption_backup":backup["path"].clone(),
            "note":"The database is now a plain SQLite file on disk."
        }))
    }

    /// Verify that a passphrase matches the stored key, without opening the
    /// database.
    pub fn encryption_verify_passphrase(&self, p: &Value) -> Result<Value> {
        let state = encryption::read_state(&self.path)?;
        if state.protection != encryption::Protection::Passphrase {
            return Err(LifeError::Invalid(
                "This database is not passphrase protected".into(),
            ));
        }
        let passphrase = require(p, "passphrase")?;
        let salt = state
            .salt
            .clone()
            .ok_or_else(|| LifeError::Invalid("The encryption state file has no salt".into()))?;
        let params = state.params.clone().unwrap_or_default();
        let expected = encryption::resolve_key_with(&state, self.key.as_deref())?;
        let matched = encryption::verify_passphrase(&passphrase, &salt, &params, &expected);
        Ok(json!({"matches":matched}))
    }
}
