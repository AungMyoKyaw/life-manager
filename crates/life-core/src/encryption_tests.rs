//! Tests for optional encryption at rest.
//!
//! Every test uses a disposable database and the passphrase path, so the
//! suite needs no system state and no user interaction.
use super::*;

fn store_at(path: &std::path::Path) -> Store {
    Store::open(path).unwrap()
}

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("life-enc-{name}-{}.sqlite", id()))
}

#[test]
fn a_plain_database_reports_no_protection() {
    let path = temp_path("plain");
    let mut s = store_at(&path);
    s.call("tasks.create", &json!({"title":"Plain task"}))
        .unwrap();
    let status = s.call("encryption.status", &json!({})).unwrap();
    assert_eq!(status["protection"], "none");
    assert_eq!(status["encrypted_on_disk"], false);
    assert!(
        status["file_header"]
            .as_str()
            .unwrap()
            .starts_with("SQLite format 3")
    );
}

#[test]
fn a_passphrase_encrypted_database_round_trips() {
    let path = temp_path("pass");
    let _ = std::fs::remove_file(&path);
    {
        let mut s = store_at(&path);
        s.call("tasks.create", &json!({"title":"Secret task"}))
            .unwrap();
        s.call(
            "encryption.enable",
            &json!({"mode":"passphrase","passphrase":"correct horse"}),
        )
        .unwrap();
    }
    // The file on disk must not contain the plaintext.
    let raw = std::fs::read(&path).unwrap();
    assert!(!raw.windows(10).any(|w| w == b"Secret task"));
    assert!(!raw.starts_with(b"SQLite format 3"));
    assert!(encryption::looks_encrypted(&path));

    // Opening without the passphrase must fail.
    assert!(Store::open(&path).is_err());
    assert!(Store::open_with_diagnostics(&path).is_err());

    // Opening with the passphrase must work and the data must be there.
    let mut s = Store::open_with_passphrase(&path, "correct horse").unwrap();
    let tasks = s.call("tasks.list", &json!({})).unwrap();
    assert_eq!(tasks.as_array().unwrap().len(), 1);
    assert_eq!(tasks[0]["title"], "Secret task");
    let status = s.call("encryption.status", &json!({})).unwrap();
    assert_eq!(status["protection"], "passphrase");
    assert_eq!(status["encrypted_on_disk"], true);
}

#[test]
fn a_wrong_passphrase_is_refused() {
    let path = temp_path("wrong");
    let _ = std::fs::remove_file(&path);
    {
        let mut s = store_at(&path);
        s.call(
            "encryption.enable",
            &json!({"mode":"passphrase","passphrase":"right one"}),
        )
        .unwrap();
    }
    assert!(Store::open_with_passphrase(&path, "wrong one").is_err());
    assert!(Store::open(&path).is_err());
}

#[test]
fn encryption_creates_a_verified_plain_backup_first() {
    let path = temp_path("backup");
    let _ = std::fs::remove_file(&path);
    let mut s = store_at(&path);
    s.call("tasks.create", &json!({"title":"Original"}))
        .unwrap();
    let result = s
        .call(
            "encryption.enable",
            &json!({"mode":"passphrase","passphrase":"p"}),
        )
        .unwrap();
    let backup = result["pre_encryption_backup"].as_str().unwrap();
    assert!(std::path::Path::new(backup).exists());
    // The backup must be a readable plain database holding the same rows.
    let mut backup_store = Store::open(backup).unwrap();
    let tasks = backup_store.call("tasks.list", &json!({})).unwrap();
    assert_eq!(tasks.as_array().unwrap().len(), 1);
    assert_eq!(tasks[0]["title"], "Original");
}

#[test]
fn an_encrypted_database_can_be_backed_up_and_restored() {
    let path = temp_path("encbackup");
    let _ = std::fs::remove_file(&path);
    let mut s = store_at(&path);
    s.call("tasks.create", &json!({"title":"Before"})).unwrap();
    s.call(
        "encryption.enable",
        &json!({"mode":"passphrase","passphrase":"p"}),
    )
    .unwrap();
    let backup = s.call("backup.create", &json!({})).unwrap();
    assert_eq!(backup["encrypted"], true);
    let backup_path = backup["path"].as_str().unwrap().to_string();
    // The encrypted backup must itself be encrypted.
    assert!(encryption::looks_encrypted(std::path::Path::new(
        &backup_path
    )));

    s.call("tasks.create", &json!({"title":"After"})).unwrap();
    // The restore reads the backup with the key, so the backup needs its own
    // protection state file beside it, and the passphrase is passed in.
    let live_state = encryption::read_state(&path).unwrap();
    let backup_state = encryption::state_path(std::path::Path::new(&backup_path));
    std::fs::write(
        &backup_state,
        serde_json::json!({
            "protection":"passphrase",
            "salt":live_state.salt,
            "params":{"memory_kib":19456,"time_cost":2,"parallelism":1}
        })
        .to_string(),
    )
    .unwrap();
    // The restore is verified against a disposable database, because the
    // restore replaces the live database file.
    let restore_target = temp_path("restore-target");
    let _ = std::fs::remove_file(&restore_target);
    {
        let mut target = store_at(&restore_target);
        target
            .call(
                "backup.restore",
                &json!({"path":backup_path,"confirm":true,"passphrase":"p"}),
            )
            .unwrap();
        let tasks = target.call("tasks.list", &json!({})).unwrap();
        let titles: Vec<String> = tasks
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["title"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(titles, vec!["Before"]);
    }
}

#[test]
fn encryption_can_be_turned_off_again() {
    let path = temp_path("off");
    let _ = std::fs::remove_file(&path);
    let mut s = store_at(&path);
    s.call("tasks.create", &json!({"title":"Plain again"}))
        .unwrap();
    s.call(
        "encryption.enable",
        &json!({"mode":"passphrase","passphrase":"p"}),
    )
    .unwrap();
    s.call("encryption.disable", &json!({"confirm":true}))
        .unwrap();
    let status = s.call("encryption.status", &json!({})).unwrap();
    assert_eq!(status["protection"], "none");
    assert_eq!(status["encrypted_on_disk"], false);
    // The data must survive the round trip.
    let tasks = s.call("tasks.list", &json!({})).unwrap();
    assert_eq!(tasks.as_array().unwrap().len(), 1);
    assert_eq!(tasks[0]["title"], "Plain again");
}

#[test]
fn a_passphrase_can_be_verified_without_opening_the_database() {
    let path = temp_path("verify");
    let _ = std::fs::remove_file(&path);
    let mut s = store_at(&path);
    s.call(
        "encryption.enable",
        &json!({"mode":"passphrase","passphrase":"p"}),
    )
    .unwrap();
    let matched = s
        .call("encryption.verify_passphrase", &json!({"passphrase":"p"}))
        .unwrap();
    assert_eq!(matched["matches"], true);
    let wrong = s
        .call(
            "encryption.verify_passphrase",
            &json!({"passphrase":"nope"}),
        )
        .unwrap();
    assert_eq!(wrong["matches"], false);
}

#[test]
fn a_plain_database_refuses_to_be_encrypted_twice() {
    let path = temp_path("twice");
    let mut s = store_at(&path);
    s.call(
        "encryption.enable",
        &json!({"mode":"passphrase","passphrase":"p"}),
    )
    .unwrap();
    assert!(
        s.call(
            "encryption.enable",
            &json!({"mode":"passphrase","passphrase":"p"})
        )
        .is_err()
    );
    assert!(s.call("encryption.disable", &json!({})).is_err());
}

#[test]
fn secrets_are_never_written_to_the_database_or_the_state_file() {
    let path = temp_path("leak");
    let _ = std::fs::remove_file(&path);
    let mut s = store_at(&path);
    s.call(
        "encryption.enable",
        &json!({"mode":"passphrase","passphrase":"my secret phrase"}),
    )
    .unwrap();
    let state = std::fs::read_to_string(encryption::state_path(&path)).unwrap();
    assert!(!state.contains("my secret phrase"));
    assert!(!state.to_lowercase().contains("passphrase\":"));
    let raw = std::fs::read(&path).unwrap();
    assert!(!raw.windows(16).any(|w| w == b"my secret phrase"));
}
