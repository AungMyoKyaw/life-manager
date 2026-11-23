//! macOS launchd-driven reminder delivery. No GUI or network dependency.
use life_core::{LifeError, Result, Store};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const LABEL: &str = "com.lifemanager.reminders";
const SCRIPT: &str =
    "on run argv\n  display notification (item 2 of argv) with title (item 1 of argv)\nend run";

#[cfg(any(target_os = "macos", test))]
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn launch_agent_path() -> Result<PathBuf> {
    let home =
        std::env::var_os("HOME").ok_or_else(|| LifeError::Invalid("HOME is unset".into()))?;
    Ok(PathBuf::from(home)
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist")))
}
fn user_domain() -> String {
    let output = Command::new("/usr/bin/id").arg("-u").output();
    let uid = output
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    format!("gui/{uid}")
}
#[cfg(any(target_os = "macos", test))]
fn plist(program: &Path, db: Option<&Path>) -> Result<String> {
    let p = program
        .to_str()
        .ok_or_else(|| LifeError::Invalid("Executable path is not UTF-8".into()))?;
    let mut args = format!(
        "<string>{}</string><string>reminder</string><string>check</string>",
        xml_escape(p)
    );
    if let Some(db) = db {
        let db = db
            .to_str()
            .ok_or_else(|| LifeError::Invalid("Database path is not UTF-8".into()))?;
        args = format!(
            "<string>{}</string><string>--db</string><string>{}</string><string>reminder</string><string>check</string>",
            xml_escape(p),
            xml_escape(db)
        );
    }
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict>\n<key>Label</key><string>{LABEL}</string>\n<key>ProgramArguments</key><array>{args}</array>\n<key>RunAtLoad</key><true/>\n<key>StartInterval</key><integer>60</integer>\n<key>ProcessType</key><string>Background</string>\n<key>StandardOutPath</key><string>/dev/null</string>\n<key>StandardErrorPath</key><string>/dev/null</string>\n</dict></plist>\n"
    ))
}
#[cfg(target_os = "macos")]
fn cmd_ok(program: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(program).args(args).output()?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(LifeError::Invalid(format!(
        "{} failed: {}",
        program,
        stderr.chars().take(500).collect::<String>()
    )))
}
fn notify(title: &str, body: &str) -> Result<()> {
    let status = Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(SCRIPT)
        .arg(title)
        .arg(body)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(LifeError::Invalid(format!(
            "macOS notification exited with {status}"
        )))
    }
}
fn store(db: Option<&Path>) -> Result<Store> {
    match db {
        Some(path) => Store::open(path),
        None => Store::default(),
    }
}

pub fn check(db: Option<&Path>, dry_run: bool) -> Result<Value> {
    #[cfg(not(target_os = "macos"))]
    if !dry_run {
        return Err(LifeError::Invalid(
            "Notification delivery is currently macOS only".into(),
        ));
    }
    let mut s = store(db)?;
    let due = s.call(
        "reminders.due",
        &json!({"forward_minutes":0,"lookback_minutes":1440}),
    )?;
    let mut delivered = 0;
    let mut failures = Vec::new();
    let mut claimed = 0;
    for item in due.as_array().into_iter().flatten() {
        let Some(key) = item["key"].as_str() else {
            continue;
        };
        if dry_run {
            continue;
        }
        let lease = s.call(
            "reminders.claim",
            &json!({"key":key,"forward_minutes":0,"lookback_minutes":1440}),
        )?;
        if lease["claimed"] != true {
            continue;
        }
        claimed += 1;
        let title = item["title"].as_str().unwrap_or("Life Manager reminder");
        let body = item["body"].as_str().unwrap_or("Upcoming commitment");
        match notify(title, body) {
            Ok(()) => {
                if let Err(e) = s.call(
                    "reminders.delivered",
                    &json!({"key":key,"actor":"reminder-agent"}),
                ) {
                    failures.push(
                        json!({"key":key,"error":e.to_string(),"delivery_state":"unconfirmed"}),
                    );
                } else {
                    delivered += 1
                }
            }
            Err(e) => {
                let _ = s.call("reminders.release", &json!({"key":key}));
                failures.push(json!({"key":key,"error":e.to_string(),"delivery_state":"released"}));
            }
        }
    }
    let result = json!({"due_count":due.as_array().map_or(0,Vec::len),"claimed":claimed,"delivered":delivered,"dry_run":dry_run,"failures":failures,"items":if dry_run {due} else {json!([])}});
    if !dry_run && !failures.is_empty() {
        return Err(LifeError::Invalid(format!(
            "{} reminder(s) failed; inspect logs",
            failures.len()
        )));
    }
    Ok(result)
}

pub fn install(db: Option<&Path>) -> Result<Value> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = db;
        Err(LifeError::Invalid(
            "launchd is supported only on macOS".into(),
        ))
    }
    #[cfg(target_os = "macos")]
    {
        let app = std::env::current_exe()?;
        let path = launch_agent_path()?;
        let contents = plist(&app, db)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?
        }
        let domain = user_domain();
        let service = format!("{domain}/{LABEL}");
        let _ = Command::new("/bin/launchctl")
            .args(["bootout", &service])
            .output();
        fs::write(&path, contents)?;
        cmd_ok(
            "/bin/launchctl",
            &[
                "bootstrap",
                &domain,
                path.to_str()
                    .ok_or_else(|| LifeError::Invalid("Invalid launch agent path".into()))?,
            ],
        )?;
        cmd_ok("/bin/launchctl", &["enable", &service])?;
        Ok(
            json!({"installed":true,"agent_path":path,"service":service,"interval_seconds":60,"database":db.map(|p|p.display().to_string()),"note":"Notifications remain opt-in; enable notification.preferences in Settings."}),
        )
    }
}
pub fn uninstall() -> Result<Value> {
    let path = launch_agent_path()?;
    let service = format!("{}/{LABEL}", user_domain());
    let _ = Command::new("/bin/launchctl")
        .args(["bootout", &service])
        .output();
    if path.exists() {
        fs::remove_file(&path)?
    }
    Ok(json!({"installed":false,"agent_path":path}))
}
pub fn status() -> Result<Value> {
    let path = launch_agent_path()?;
    let service = format!("{}/{LABEL}", user_domain());
    let active = Command::new("/bin/launchctl")
        .args(["print", &service])
        .output()
        .is_ok_and(|o| o.status.success());
    Ok(
        json!({"installed":path.is_file(),"loaded":active,"agent_path":path,"service":service,"poll_seconds":60}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plist_is_correct_and_path_safe() {
        let content = plist(
            Path::new("/tmp/life & test"),
            Some(Path::new("/tmp/my <db>.sqlite")),
        )
        .unwrap();
        assert!(content.contains("/tmp/life &amp; test"));
        assert!(content.contains("/tmp/my &lt;db&gt;.sqlite"));
        assert!(content.contains("<key>StartInterval</key><integer>60</integer>"));
        assert!(content.contains("<string>--db</string>"));
    }
    #[test]
    fn default_plist_has_no_database_argument() {
        let content = plist(Path::new("/tmp/life"), None).unwrap();
        assert!(!content.contains("--db"));
        assert!(content.contains("<string>reminder</string><string>check</string>"));
    }
}
