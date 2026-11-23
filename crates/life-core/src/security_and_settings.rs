//! Keyboard shortcuts, integration accounts, remote MCP clients, notification
//! authorization and the secret-safe redaction rules.
use super::*;

const DEFAULT_SHORTCUTS: &[(&str, &str, &str)] = &[
    ("command_palette", "Meta+K", "Open the command palette"),
    ("quick_capture", "Meta+N", "Capture a new task"),
    ("search", "Meta+Shift+F", "Search the workspace"),
    ("today", "Meta+1", "Go to Today"),
    ("tasks", "Meta+2", "Go to Tasks"),
    ("calendar", "Meta+3", "Go to Calendar"),
    ("focus", "Meta+4", "Go to Focus"),
    ("settings", "Meta+Comma", "Open Settings"),
    ("close_dialog", "Escape", "Close the open dialog"),
];

impl Store {
    // --- keyboard shortcuts -------------------------------------------------

    pub fn shortcuts_list(&self) -> Result<Value> {
        let rows = self.rows("SELECT * FROM keyboard_shortcuts ORDER BY action", &[])?;
        if rows.as_array().is_some_and(|list| list.is_empty()) {
            let mut defaults: Vec<Value> = Vec::new();
            for (action, keys, label) in DEFAULT_SHORTCUTS {
                let key = id();
                self.conn.execute(
                    "INSERT OR IGNORE INTO keyboard_shortcuts(id,action,keys,label,updated_at) VALUES(?1,?2,?3,?4,?5)",
                    params![key, action, keys, label, now()],
                )?;
                defaults.push(json!({"id":key,"action":action,"keys":keys,"label":label}));
            }
            return Ok(json!(defaults));
        }
        Ok(rows)
    }

    pub fn shortcuts_update(&self, p: &Value, actor: &str) -> Result<Value> {
        let action = require(p, "action")?;
        let keys = require(p, "keys")?;
        let existing = self
            .rows(
                "SELECT id FROM keyboard_shortcuts WHERE action=?1",
                &[&action],
            )?
            .as_array()
            .and_then(|rows| rows.first())
            .and_then(|row| row["id"].as_str())
            .map(|v| v.to_string());
        let conflict = self.rows(
            "SELECT action FROM keyboard_shortcuts WHERE keys=?1 AND action<>?2",
            &[&keys, &action],
        )?;
        if conflict.as_array().is_some_and(|rows| !rows.is_empty()) {
            return Err(LifeError::Conflict(format!(
                "Those keys are already bound to {}",
                conflict[0]["action"].as_str().unwrap_or("another action")
            )));
        }
        let key = existing.unwrap_or_else(id);
        self.conn.execute(
            "INSERT INTO keyboard_shortcuts(id,action,keys,label,updated_at) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(action) DO UPDATE SET keys=excluded.keys,label=excluded.label,updated_at=excluded.updated_at",
            params![
                key,
                action,
                keys,
                optional(p, "label").unwrap_or_default(),
                now()
            ],
        )?;
        self.record(actor, "shortcuts.update", Some(&key), p)?;
        self.rows(
            "SELECT * FROM keyboard_shortcuts WHERE action=?1",
            &[&action],
        )
        .map(|v| v[0].clone())
    }

    pub fn shortcuts_reset(&self, p: &Value, actor: &str) -> Result<Value> {
        self.conn.execute("DELETE FROM keyboard_shortcuts", [])?;
        for (action, keys, label) in DEFAULT_SHORTCUTS {
            self.conn.execute(
                "INSERT INTO keyboard_shortcuts(id,action,keys,label,updated_at) VALUES(?1,?2,?3,?4,?5)",
                params![id(), action, keys, label, now()],
            )?;
        }
        self.record(actor, "shortcuts.reset", None, p)?;
        self.shortcuts_list()
    }

    // --- integration accounts -------------------------------------------------

    pub fn integrations_list(&self) -> Result<Value> {
        let rows = self.rows(
            "SELECT id,provider,account_label,status,scopes,last_sync_at,created_at,updated_at FROM integration_accounts ORDER BY provider,account_label",
            &[],
        )?;
        let mut accounts: Vec<Value> = rows.as_array().into_iter().flatten().cloned().collect();
        for provider in ["google_calendar", "apple_calendar", "caldav", "github"] {
            if !accounts
                .iter()
                .any(|row| row["provider"].as_str() == Some(provider))
            {
                accounts.push(json!({
                    "provider":provider,
                    "account_label":"",
                    "status":"not configured",
                    "scopes":[],
                    "last_sync_at":null
                }));
            }
        }
        Ok(json!(accounts))
    }

    pub fn integrations_set_status(&self, p: &Value, actor: &str) -> Result<Value> {
        let provider = require(p, "provider")?;
        let status = optional(p, "status").unwrap_or_else(|| "disconnected".into());
        if ![
            "not configured",
            "disconnected",
            "authorized",
            "syncing",
            "error",
        ]
        .contains(&status.as_str())
        {
            return Err(LifeError::Invalid(
                "status must be not configured, disconnected, authorized, syncing or error".into(),
            ));
        }
        let label = optional(p, "account_label").unwrap_or_default();
        let key = id();
        self.conn.execute(
            "INSERT INTO integration_accounts(id,provider,account_label,status,scopes,last_sync_at,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?7) ON CONFLICT(provider,account_label) DO UPDATE SET status=excluded.status,scopes=excluded.scopes,last_sync_at=excluded.last_sync_at,updated_at=excluded.updated_at",
            params![
                key,
                provider,
                label,
                status,
                p.get("scopes")
                    .cloned()
                    .unwrap_or_else(|| json!([]))
                    .to_string(),
                optional(p, "last_sync_at"),
                now()
            ],
        )?;
        self.record(actor, "integrations.status", Some(&provider), p)?;
        self.rows(
            "SELECT provider,account_label,status,scopes,last_sync_at FROM integration_accounts WHERE provider=?1 AND account_label=?2",
            &[&provider, &label],
        )
        .map(|v| v[0].clone())
    }

    /// The configuration an integration needs before it can run. Nothing is
    /// invented: missing values are reported as missing.
    pub fn integrations_configure(&self, p: &Value) -> Result<Value> {
        let provider = require(p, "provider")?;
        let mut required: Vec<&str> = Vec::new();
        let mut present: Vec<&str> = Vec::new();
        let mut missing: Vec<&str> = Vec::new();
        let mut missing_labels: Vec<&str> = Vec::new();
        let names: Vec<(&str, &str)> = match provider.as_str() {
            "google_calendar" => vec![
                ("LIFE_GOOGLE_CLIENT_ID", "Google OAuth client id"),
                ("LIFE_GOOGLE_CLIENT_SECRET", "Google OAuth client secret"),
            ],
            "github" => vec![
                ("LIFE_GITHUB_CLIENT_ID", "GitHub OAuth client id"),
                ("LIFE_GITHUB_CLIENT_SECRET", "GitHub OAuth client secret"),
            ],
            "caldav" => vec![
                ("LIFE_CALDAV_URL", "CalDAV server URL"),
                ("LIFE_CALDAV_USER", "CalDAV user name"),
                ("LIFE_CALDAV_PASSWORD", "CalDAV password"),
            ],
            "apple_calendar" => vec![(
                "LIFE_APPLE_CALENDAR_ACCESS",
                "Apple Calendar access granted in macOS Settings",
            )],
            other => {
                return Err(LifeError::Invalid(format!("Unsupported provider {other}")));
            }
        };
        for (name, label) in names {
            required.push(name);
            if std::env::var(name).is_ok_and(|value| !value.trim().is_empty()) {
                present.push(name);
            } else {
                missing.push(name);
                missing_labels.push(label);
            }
        }
        Ok(json!({
            "provider":provider,
            "required":required,
            "present":present,
            "missing":missing,
            "missing_labels":missing_labels,
            "configured":missing.is_empty(),
            "note":"Credentials are read from the environment at run time. Life Manager never stores a provider secret in the database."
        }))
    }

    // --- remote MCP clients ----------------------------------------------------

    /// Hash a bearer token for storage. The plaintext is never written.
    pub fn hash_token(token: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"life-manager-remote-mcp:");
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn remote_clients_list(&self) -> Result<Value> {
        self.rows(
            "SELECT id,name,scopes,created_at,last_used_at,revoked_at FROM remote_mcp_clients ORDER BY created_at DESC",
            &[],
        )
    }

    pub fn remote_clients_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let name = require(p, "name")?;
        let token = require(p, "token")?;
        if token.chars().count() < 24 {
            return Err(LifeError::Invalid(
                "A remote client token must be at least 24 characters".into(),
            ));
        }
        let key = id();
        self.conn.execute(
            "INSERT INTO remote_mcp_clients(id,name,token_hash,scopes,created_at) VALUES(?1,?2,?3,?4,?5)",
            params![
                key,
                name,
                Self::hash_token(&token),
                p.get("scopes")
                    .cloned()
                    .unwrap_or_else(|| json!([]))
                    .to_string(),
                now()
            ],
        )?;
        self.record(actor, "remote_clients.create", Some(&key), p)?;
        Ok(
            json!({"id":key,"name":name,"scopes":p.get("scopes").cloned().unwrap_or_else(|| json!([]))}),
        )
    }

    pub fn remote_clients_revoke(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let n = self.conn.execute(
            "UPDATE remote_mcp_clients SET revoked_at=?2 WHERE id=?1 AND revoked_at IS NULL",
            params![key, now()],
        )?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "remote_clients.revoke", Some(&key), p)?;
        Ok(json!({"id":key,"revoked":true}))
    }

    /// Check a bearer token against the stored client hashes.
    pub fn remote_client_authorised(&self, token: &str) -> Result<Option<String>> {
        let hash = Self::hash_token(token);
        let rows = self.rows(
            "SELECT id,name FROM remote_mcp_clients WHERE token_hash=?1 AND revoked_at IS NULL",
            &[&hash],
        )?;
        let name = rows
            .as_array()
            .and_then(|r| r.first())
            .and_then(|r| r["name"].as_str())
            .map(|v| v.to_string());
        if name.is_some() {
            self.conn.execute(
                "UPDATE remote_mcp_clients SET last_used_at=?2 WHERE token_hash=?1",
                params![&hash, now()],
            )?;
        }
        Ok(name)
    }

    // --- notification authorization ---------------------------------------------

    pub fn notification_authorization_set(&self, p: &Value, actor: &str) -> Result<Value> {
        let bundle = optional(p, "bundle_id").unwrap_or_else(|| "app.lifemanager.desktop".into());
        let status = optional(p, "status").unwrap_or_else(|| "unknown".into());
        if !["authorized", "denied", "not determined", "unknown"].contains(&status.as_str()) {
            return Err(LifeError::Invalid(
                "status must be authorized, denied, not determined or unknown".into(),
            ));
        }
        self.conn.execute(
            "INSERT INTO notification_authorization(id,bundle_id,status,checked_at) VALUES(?1,?2,?3,?4) ON CONFLICT(bundle_id) DO UPDATE SET status=excluded.status,checked_at=excluded.checked_at",
            params![id(), bundle, status, now()],
        )?;
        self.record(actor, "notification_authorization.set", Some(&bundle), p)?;
        self.rows(
            "SELECT bundle_id,status,checked_at FROM notification_authorization WHERE bundle_id=?1",
            &[&bundle],
        )
        .map(|v| v[0].clone())
    }

    pub fn notification_authorization_get(&self) -> Result<Value> {
        let rows = self.rows(
            "SELECT bundle_id,status,checked_at FROM notification_authorization ORDER BY checked_at DESC LIMIT 1",
            &[],
        )?;
        Ok(rows.as_array().and_then(|r| r.first()).cloned().unwrap_or(
            json!({"bundle_id":"app.lifemanager.desktop","status":"unknown","checked_at":null}),
        ))
    }

    // --- redaction ---------------------------------------------------------------

    /// Remove anything that looks like a secret from a value before it is
    /// written to the activity ledger.
    pub fn redact(&self, value: &Value) -> Value {
        const SECRETS: [&str; 10] = [
            "access_token",
            "refresh_token",
            "client_secret",
            "password",
            "api_key",
            "apikey",
            "token",
            "secret",
            "authorization",
            "bearer",
        ];
        match value {
            Value::Object(map) => {
                let mut out = serde_json::Map::new();
                for (key, inner) in map {
                    let lowered = key.to_lowercase();
                    if SECRETS.iter().any(|needle| lowered.contains(needle)) {
                        out.insert(key.clone(), json!("[redacted]"));
                    } else {
                        out.insert(key.clone(), self.redact(inner));
                    }
                }
                Value::Object(out)
            }
            Value::Array(items) => Value::Array(items.iter().map(|v| self.redact(v)).collect()),
            other => other.clone(),
        }
    }
}
