//! Optional encryption at rest for the local database.
//!
//! The store opens a plain SQLite database by default. When a user turns
//! encryption on, the database becomes a SQLCipher database keyed by a random
//! 32-byte master key. The key is either supplied directly through the
//! environment, or derived from a passphrase with Argon2id.
//!
//! Nothing here invents a key, and no key is stored outside the process.
//! If the key cannot be found the database stays closed and the error
//! explains exactly what is missing.
use super::{LifeError, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use rusqlite::Connection;
use std::path::Path;

/// The file that records how the database is protected.
const STATE_FILE: &str = "encryption.json";
const ARGON_MEMORY_KIB: u32 = 19 * 1024;
const ARGON_TIME_COST: u32 = 2;
const ARGON_PARALLELISM: u32 = 1;

/// How the database is protected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Protection {
    /// The database is a plain SQLite file.
    None,
    /// The database is encrypted with a key derived from a passphrase.
    Passphrase,
}

impl Protection {
    pub fn label(&self) -> &'static str {
        match self {
            Protection::None => "not encrypted",
            Protection::Passphrase => "encrypted with a passphrase",
        }
    }
}

/// The recorded protection state of a database.
#[derive(Clone, Debug)]
pub struct EncryptionState {
    pub protection: Protection,
    /// The Argon2 salt, hex encoded, for a passphrase-protected database.
    pub salt: Option<String>,
    /// The Argon2 parameters, so the parameters can change later.
    pub params: Option<Argon2Params>,
}

#[derive(Clone, Debug)]
pub struct Argon2Params {
    pub memory_kib: u32,
    pub time_cost: u32,
    pub parallelism: u32,
}

impl Default for Argon2Params {
    fn default() -> Self {
        Self {
            memory_kib: ARGON_MEMORY_KIB,
            time_cost: ARGON_TIME_COST,
            parallelism: ARGON_PARALLELISM,
        }
    }
}

/// The path of the state file that sits beside a database.
pub fn state_path(database: &Path) -> std::path::PathBuf {
    let name = database
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "life.sqlite".to_string());
    database.with_file_name(format!("{name}.{STATE_FILE}"))
}

/// Read the recorded protection state of a database.
pub fn read_state(database: &Path) -> Result<EncryptionState> {
    let path = state_path(database);
    if !path.exists() {
        return Ok(EncryptionState {
            protection: Protection::None,
            salt: None,
            params: None,
        });
    }
    let raw = std::fs::read_to_string(&path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| LifeError::Invalid(format!("The encryption state file is unreadable: {e}")))?;
    let protection = match value["protection"].as_str().unwrap_or("none") {
        "passphrase" => Protection::Passphrase,
        _ => Protection::None,
    };
    let params = value["params"].as_object().map(|object| Argon2Params {
        memory_kib: object["memory_kib"]
            .as_u64()
            .unwrap_or(ARGON_MEMORY_KIB as u64) as u32,
        time_cost: object["time_cost"]
            .as_u64()
            .unwrap_or(ARGON_TIME_COST as u64) as u32,
        parallelism: object["parallelism"]
            .as_u64()
            .unwrap_or(ARGON_PARALLELISM as u64) as u32,
    });
    Ok(EncryptionState {
        protection,
        salt: value["salt"].as_str().map(|v| v.to_string()),
        params,
    })
}

/// Record the protection state of a database.
pub fn write_state(database: &Path, state: &EncryptionState) -> Result<()> {
    let mut params = serde_json::Map::new();
    if let Some(p) = &state.params {
        params.insert("memory_kib".into(), serde_json::json!(p.memory_kib));
        params.insert("time_cost".into(), serde_json::json!(p.time_cost));
        params.insert("parallelism".into(), serde_json::json!(p.parallelism));
    }
    let protection = match state.protection {
        Protection::None => "none",
        Protection::Passphrase => "passphrase",
    };
    let document = serde_json::json!({
        "protection":protection,
        "salt":state.salt,
        "params":serde_json::Value::Object(params),
        "cipher":"sqlcipher-4",
        "page_size":4096,
        "hmac":"HMAC_SHA512",
        "kdf":"PBKDF2_HMAC_SHA512"
    });
    std::fs::write(
        state_path(database),
        serde_json::to_string_pretty(&document)?,
    )?;
    Ok(())
}

/// Remove the recorded protection state, for example after a database is
/// decrypted back to a plain file.
pub fn clear_state(database: &Path) -> Result<()> {
    let path = state_path(database);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// True when the file looks like an encrypted SQLCipher database.
pub fn looks_encrypted(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    if bytes.len() < 16 {
        return false;
    }
    &bytes[..15] != b"SQLite format 3"
}

/// A 32-byte master key, hex encoded.
pub type MasterKey = String;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(value: &str) -> Result<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return Err(LifeError::Invalid("The master key is not valid hex".into()));
    }
    (0..value.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&value[i..i + 2], 16)
                .map_err(|_| LifeError::Invalid("The master key is not valid hex".into()))
        })
        .collect()
}

/// Cryptographically random bytes, taken from the operating system.
pub fn random_bytes(count: usize) -> Result<Vec<u8>> {
    let mut bytes = vec![0u8; count];
    getrandom::fill(&mut bytes)
        .map_err(|e| LifeError::Invalid(format!("Secure random bytes are unavailable: {e}")))?;
    Ok(bytes)
}

/// Derive a 32-byte master key from a passphrase with Argon2id.
///
/// The raw output of the KDF is the key, so the database never sees the
/// passphrase itself and the key never leaves this function in the clear.
pub fn derive_key(passphrase: &str, salt_hex: &str, params: &Argon2Params) -> Result<MasterKey> {
    let salt_bytes = unhex(salt_hex)?;
    if salt_bytes.len() < 8 {
        return Err(LifeError::Invalid("The salt is too short".into()));
    }
    let argon = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(
            params.memory_kib,
            params.time_cost,
            params.parallelism,
            None,
        )
        .map_err(|e| LifeError::Invalid(format!("Invalid Argon2 parameters: {e}")))?,
    );
    let mut key = [0u8; 32];
    argon
        .hash_password_into(passphrase.as_bytes(), &salt_bytes, &mut key)
        .map_err(|e| LifeError::Invalid(format!("The passphrase could not be derived: {e}")))?;
    Ok(hex(&key))
}

/// Verify a passphrase against a derived key without opening the database.
pub fn verify_passphrase(
    passphrase: &str,
    salt_hex: &str,
    params: &Argon2Params,
    expected_key: &MasterKey,
) -> bool {
    match derive_key(passphrase, salt_hex, params) {
        Ok(derived) => constant_time_eq(derived.as_bytes(), expected_key.as_bytes()),
        Err(_) => false,
    }
}

/// Compare two byte strings without an early exit.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        difference |= a ^ b;
    }
    difference == 0
}

/// The pragmas that put a SQLCipher connection into its configured state.
pub fn cipher_pragmas(key: &MasterKey) -> String {
    format!(
        "PRAGMA cipher_page_size = 4096; \
         PRAGMA cipher_hmac_algorithm = HMAC_SHA512; \
         PRAGMA cipher_kdf_algorithm = PBKDF2_HMAC_SHA512; \
         PRAGMA key = \"x'{key}'\";"
    )
}

/// Open a connection with the cipher pragmas applied.
pub fn open_encrypted(path: &Path, key: &MasterKey) -> Result<Connection> {
    let connection = Connection::open(path)?;
    connection.execute_batch(&cipher_pragmas(key))?;
    Ok(connection)
}

/// Resolve the master key for a database.
///
/// The raw key is read from the environment when it is present. Otherwise the
/// key is derived from the passphrase in the environment with Argon2id.
pub fn resolve_key(state: &EncryptionState) -> Result<MasterKey> {
    resolve_key_with(state, None)
}

/// Resolve the master key of a source database, with an explicit passphrase
/// when the caller supplies one.
pub fn resolve_source_key(
    state: &EncryptionState,
    cached: Option<&str>,
    passphrase: Option<&str>,
) -> Result<MasterKey> {
    if let Some(key) = cached {
        return Ok(key.to_string());
    }
    if state.protection == Protection::None {
        return Err(LifeError::Invalid("The database is not encrypted".into()));
    }
    if let Ok(value) = std::env::var("LIFE_DB_KEY") {
        let value = value.trim().to_string();
        if !value.is_empty() {
            return Ok(value);
        }
    }
    let secret = match passphrase {
        Some(value) => value.to_string(),
        None => std::env::var("LIFE_DB_PASSPHRASE").map_err(|_| {
            LifeError::Invalid(
                "This database needs a passphrase. Pass it in, or set LIFE_DB_PASSPHRASE.".into(),
            )
        })?,
    };
    let salt = state
        .salt
        .clone()
        .ok_or_else(|| LifeError::Invalid("The encryption state file has no salt".into()))?;
    let params = state.params.clone().unwrap_or_default();
    derive_key(&secret, &salt, &params)
}

/// Resolve the master key, preferring a key this session already holds.
pub fn resolve_key_with(state: &EncryptionState, cached: Option<&str>) -> Result<MasterKey> {
    if let Some(key) = cached {
        return Ok(key.to_string());
    }
    match state.protection {
        Protection::None => Err(LifeError::Invalid("The database is not encrypted".into())),
        Protection::Passphrase => {
            if let Ok(value) = std::env::var("LIFE_DB_KEY") {
                let value = value.trim().to_string();
                if !value.is_empty() {
                    return Ok(value);
                }
            }
            let passphrase = std::env::var("LIFE_DB_PASSPHRASE").map_err(|_| {
                LifeError::Invalid(
                    "This database needs a passphrase. Set LIFE_DB_PASSPHRASE for this command."
                        .into(),
                )
            })?;
            let salt = state.salt.clone().ok_or_else(|| {
                LifeError::Invalid("The encryption state file has no salt".into())
            })?;
            let params = state.params.clone().unwrap_or_default();
            derive_key(&passphrase, &salt, &params)
        }
    }
}

/// The salt for a new passphrase-protected database.
pub fn new_salt() -> Result<String> {
    Ok(hex(&random_bytes(16)?))
}
