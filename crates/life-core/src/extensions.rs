use super::*;
use chrono::{Datelike, Weekday};

impl Store {
    pub fn habit_history(&self, p: &Value) -> Result<Value> {
        let start = optional(p, "from")
            .unwrap_or_else(|| (Local::now().date_naive() - Duration::days(30)).to_string());
        let end = optional(p, "to").unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
        self.rows("SELECT l.*,h.title AS habit_title FROM habit_logs l JOIN habits h ON h.id=l.habit_id WHERE l.day>=?1 AND l.day<=?2 ORDER BY l.day DESC,h.title",&[&start,&end])
    }
    pub fn pause_focus(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let old = self.fetch("focus_sessions", &key)?;
        if old["status"] != "active" {
            return Err(LifeError::Conflict("Session is not active".into()));
        }
        let ts = now();
        self.conn.execute(
            "INSERT INTO focus_pauses(id,session_id,started_at) VALUES(?1,?2,?3)",
            params![id(), key, ts],
        )?;
        self.conn.execute(
            "UPDATE focus_sessions SET status='paused' WHERE id=?1",
            [&key],
        )?;
        self.record(actor, "focus.pause", Some(&key), p)?;
        self.fetch("focus_sessions", &key)
    }
    pub fn resume_focus(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let old = self.fetch("focus_sessions", &key)?;
        if old["status"] != "paused" {
            return Err(LifeError::Conflict("Session is not paused".into()));
        }
        self.conn.execute(
            "UPDATE focus_pauses SET ended_at=?2 WHERE session_id=?1 AND ended_at IS NULL",
            params![key, now()],
        )?;
        self.conn.execute(
            "UPDATE focus_sessions SET status='active' WHERE id=?1",
            [&key],
        )?;
        self.record(actor, "focus.resume", Some(&key), p)?;
        self.fetch("focus_sessions", &key)
    }
    pub fn routines(&self) -> Result<Value> {
        self.rows(
            "SELECT * FROM routines WHERE archived=0 ORDER BY created_at DESC",
            &[],
        )
    }
    pub fn create_routine(&self, p: &Value, actor: &str) -> Result<Value> {
        let title = require(p, "title")?;
        let key = id();
        self.conn.execute(
            "INSERT INTO routines(id,title,description,created_at) VALUES(?1,?2,?3,?4)",
            params![key, title, text(p, "description"), now()],
        )?;
        self.record(actor, "routines.create", Some(&key), p)?;
        self.rows("SELECT * FROM routines WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }
    pub fn update_routine(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let current = self.rows("SELECT * FROM routines WHERE id=?1", &[&key])?;
        if current.as_array().is_none_or(|v| v.is_empty()) {
            return Err(LifeError::NotFound(key));
        }
        let title = optional(p, "title")
            .unwrap_or_else(|| current[0]["title"].as_str().unwrap_or("").into());
        let description = optional(p, "description")
            .unwrap_or_else(|| current[0]["description"].as_str().unwrap_or("").into());
        self.conn.execute(
            "UPDATE routines SET title=?2,description=?3 WHERE id=?1",
            params![key, title, description],
        )?;
        self.record(actor, "routines.update", Some(&key), p)?;
        Ok(json!({"id":key,"title":title,"description":description}))
    }
    pub fn routine_steps(&self, p: &Value) -> Result<Value> {
        let routine = require(p, "routine_id")?;
        self.rows(
            "SELECT * FROM routine_steps WHERE routine_id=?1 ORDER BY position,id",
            &[&routine],
        )
    }
    pub fn routine_step_add(&self, p: &Value, actor: &str) -> Result<Value> {
        let routine = require(p, "routine_id")?;
        let title = require(p, "title")?;
        let key = id();
        let pos = integer(p, "position", 100);
        self.conn.execute("INSERT INTO routine_steps(id,routine_id,title,position,habit_id,duration_minutes) VALUES(?1,?2,?3,?4,?5,?6)",params![key,routine,title,pos,optional(p,"habit_id"),integer(p,"duration_minutes",0)])?;
        self.record(actor, "routines.step.add", Some(&key), p)?;
        Ok(json!({"id":key,"routine_id":routine,"title":title,"position":pos}))
    }
    pub fn delete_routine(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        if self
            .conn
            .execute("UPDATE routines SET archived=1 WHERE id=?1", [&key])?
            == 0
        {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "routines.delete", Some(&key), p)?;
        Ok(json!({"id":key,"archived":true}))
    }
    pub fn milestones(&self, p: &Value) -> Result<Value> {
        let key = text(p, "project_id");
        self.rows(
            "SELECT * FROM milestones WHERE ?1='' OR project_id=?1 ORDER BY due_date,title",
            &[&key],
        )
    }
    pub fn create_milestone(&self, p: &Value, actor: &str) -> Result<Value> {
        let project = require(p, "project_id")?;
        self.fetch("projects", &project)?;
        let title = require(p, "title")?;
        let key = id();
        self.conn.execute("INSERT INTO milestones(id,project_id,title,due_date,created_at) VALUES(?1,?2,?3,?4,?5)",params![key,project,title,optional(p,"due_date"),now()])?;
        self.record(actor, "milestones.create", Some(&key), p)?;
        Ok(
            json!({"id":key,"project_id":project,"title":title,"status":"todo","due_date":optional(p,"due_date")}),
        )
    }
    pub fn complete_milestone(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        if self
            .conn
            .execute("UPDATE milestones SET status='done' WHERE id=?1", [&key])?
            == 0
        {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "milestones.complete", Some(&key), p)?;
        Ok(json!({"id":key,"status":"done"}))
    }
    pub fn reviews(&self, p: &Value) -> Result<Value> {
        let period = text(p, "period");
        self.rows(
            "SELECT * FROM reviews WHERE ?1='' OR period=?1 ORDER BY day DESC LIMIT 100",
            &[&period],
        )
    }
    pub fn create_review(&self, p: &Value, actor: &str) -> Result<Value> {
        let title = require(p, "title")?;
        let period = optional(p, "period").unwrap_or_else(|| "daily".into());
        if !["daily", "weekly", "monthly"].contains(&period.as_str()) {
            return Err(LifeError::Invalid("Unknown review period".into()));
        }
        let day = optional(p, "day").unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
        NaiveDate::parse_from_str(&day, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("Invalid review date".into()))?;
        let key = id();
        let ts = now();
        self.conn.execute("INSERT INTO reviews(id,period,day,title,content,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?6)",params![key,period,day,title,text(p,"content"),ts])?;
        self.record(actor, "reviews.create", Some(&key), p)?;
        Ok(json!({"id":key,"period":period,"day":day,"title":title}))
    }
    pub fn update_review(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        if self.conn.execute(
            "UPDATE reviews SET content=?2,updated_at=?3 WHERE id=?1",
            params![key, require(p, "content")?, now()],
        )? == 0
        {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "reviews.update", Some(&key), p)?;
        Ok(json!({"id":key,"updated":true}))
    }
    pub fn dependencies(&self, p: &Value) -> Result<Value> {
        let key = require(p, "task_id")?;
        self.rows("SELECT d.*,a.title AS task_title,b.title AS blocks_title FROM task_dependencies d JOIN tasks a ON a.id=d.task_id JOIN tasks b ON b.id=d.blocks_id WHERE d.task_id=?1 OR d.blocks_id=?1",&[&key])
    }
    pub fn add_dependency(&self, p: &Value, actor: &str) -> Result<Value> {
        let task = require(p, "task_id")?;
        let blocks = require(p, "blocks_id")?;
        if task == blocks {
            return Err(LifeError::Invalid(
                "Tasks cannot depend on themselves".into(),
            ));
        }
        self.fetch("tasks", &task)?;
        self.fetch("tasks", &blocks)?;
        let cycle:i64=self.conn.query_row(
   "WITH RECURSIVE reach(id) AS (SELECT blocks_id FROM task_dependencies WHERE task_id=?1 UNION SELECT d.blocks_id FROM task_dependencies d JOIN reach r ON d.task_id=r.id) SELECT EXISTS(SELECT 1 FROM reach WHERE id=?2)",params![blocks,task],|r|r.get(0))?;
        if cycle != 0 {
            return Err(LifeError::Conflict(
                "Dependency would create a cycle".into(),
            ));
        }
        self.conn.execute(
            "INSERT OR IGNORE INTO task_dependencies(task_id,blocks_id) VALUES(?1,?2)",
            params![task, blocks],
        )?;
        self.record(actor, "dependencies.add", Some(&task), p)?;
        Ok(json!({"task_id":task,"blocks_id":blocks}))
    }
    pub fn remove_dependency(&self, p: &Value, actor: &str) -> Result<Value> {
        let task = require(p, "task_id")?;
        let blocks = require(p, "blocks_id")?;
        self.conn.execute(
            "DELETE FROM task_dependencies WHERE task_id=?1 AND blocks_id=?2",
            params![task, blocks],
        )?;
        self.record(actor, "dependencies.remove", Some(&task), p)?;
        Ok(json!({"removed":true}))
    }
    pub fn automation_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let title = require(p, "title")?;
        let action = require(p, "action")?;
        let allowed = [
            "tasks.create",
            "notes.create",
            "habits.log",
            "reviews.create",
        ];
        if !allowed.contains(&action.as_str()) {
            return Err(LifeError::Invalid("Automation action not supported".into()));
        }
        let key = id();
        let args = p.get("arguments").cloned().unwrap_or(json!({}));
        if !args.is_object() {
            return Err(LifeError::Invalid("arguments must be an object".into()));
        }
        let trigger = optional(p, "trigger_kind").unwrap_or_else(|| "manual".into());
        self.conn.execute("INSERT INTO automation_rules(id,title,trigger_kind,action,arguments,enabled,created_at) VALUES(?1,?2,?3,?4,?5,1,?6)",params![key,title,trigger,action,args.to_string(),now()])?;
        self.record(actor, "automations.create", Some(&key), p)?;
        Ok(json!({"id":key,"title":title,"action":action,"arguments":args,"trigger_kind":trigger}))
    }
    pub fn automation_update(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let enabled = p
            .get("enabled")
            .and_then(Value::as_bool)
            .ok_or_else(|| LifeError::Invalid("enabled Boolean required".into()))?;
        if self.conn.execute(
            "UPDATE automation_rules SET enabled=?2 WHERE id=?1",
            params![key, enabled],
        )? == 0
        {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "automations.update", Some(&key), p)?;
        Ok(json!({"id":key,"enabled":enabled}))
    }
    pub fn automation_run(&mut self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let rule = self.rows("SELECT * FROM automation_rules WHERE id=?1", &[&key])?;
        let rule = rule
            .as_array()
            .and_then(|r| r.first())
            .ok_or_else(|| LifeError::NotFound(key.clone()))?;
        if rule["enabled"].as_i64() != Some(1) {
            return Err(LifeError::Conflict("Automation is disabled".into()));
        }
        let action = rule["action"]
            .as_str()
            .ok_or_else(|| LifeError::Invalid("Invalid rule".into()))?
            .to_string();
        let args: Value = serde_json::from_str(rule["arguments"].as_str().unwrap_or("{}"))?;
        let result = self.dispatch(&action, &args, &format!("automation:{actor}"))?;
        self.conn.execute(
            "UPDATE automation_rules SET last_run=?2 WHERE id=?1",
            params![key, now()],
        )?;
        self.conn.execute("INSERT INTO automation_runs(id,rule_id,status,started_at,output) VALUES(?1,?2,'completed',?3,?4)",params![id(),key,now(),result.to_string()])?;
        self.record(actor, "automations.run", Some(&key), p)?;
        Ok(json!({"rule_id":key,"action":action,"result":result}))
    }
    pub fn apply_schedule(&self, p: &Value, actor: &str) -> Result<Value> {
        let slots = p
            .get("slots")
            .and_then(Value::as_array)
            .ok_or_else(|| LifeError::Invalid("slots must be an array".into()))?;
        if slots.len() > 100 {
            return Err(LifeError::Invalid(
                "A schedule can contain at most 100 blocks".into(),
            ));
        }
        let mut windows: Vec<(String, String)> = Vec::new();
        let mut created = Vec::new();
        for slot in slots {
            let task = require(slot, "task_id")?;
            let canonical = self.fetch("tasks", &task)?;
            let start = require(slot, "start_at")?;
            let end = require(slot, "end_at")?;
            if end <= start {
                return Err(LifeError::Invalid(
                    "A block must end after it starts".into(),
                ));
            }
            if windows.iter().any(|(a, b)| start < *b && end > *a) {
                return Err(LifeError::Conflict("Overlapping proposed blocks".into()));
            }
            let existing: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM events WHERE start_at<?2 AND end_at>?1",
                params![start, end],
                |r| r.get(0),
            )?;
            if existing != 0 {
                return Err(LifeError::Conflict(format!(
                    "Conflicting calendar event at {start}"
                )));
            }
            windows.push((start.clone(), end.clone()));
            let key = id();
            let timestamp = now();
            let title = canonical["title"]
                .as_str()
                .unwrap_or("Focus block")
                .to_owned();
            self.conn.execute("INSERT INTO events(id,title,start_at,end_at,kind,task_id,color,created_at,updated_at) VALUES(?1,?2,?3,?4,'block',?5,'#DFEBF2',?6,?6)",params![key,title,start,end,task,timestamp])?;
            created.push(key);
        }
        self.record(actor, "schedule.apply", None, p)?;
        Ok(json!({"created":created,"count":created.len(),"status":"applied"}))
    }
    pub fn recurrence_next(&self, current: &Value) -> Option<String> {
        let r = current["recurrence"].as_str().unwrap_or("");
        let date = current["due_date"]
            .as_str()
            .and_then(|v| v.get(..10))
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .unwrap_or_else(|| Local::now().date_naive());
        let next = match r {
            "daily" => date + Duration::days(1),
            "weekdays" => {
                let mut d = date + Duration::days(1);
                while matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
                    d += Duration::days(1);
                }
                d
            }
            "weekly" => date + Duration::days(7),
            "monthly" => {
                let y = if date.month() == 12 {
                    date.year() + 1
                } else {
                    date.year()
                };
                let m = if date.month() == 12 {
                    1
                } else {
                    date.month() + 1
                };
                let max = (NaiveDate::from_ymd_opt(
                    if m == 12 { y + 1 } else { y },
                    if m == 12 { 1 } else { m + 1 },
                    1,
                )? - Duration::days(1))
                .day();
                NaiveDate::from_ymd_opt(y, m, date.day().min(max))?
            }
            _ => return None,
        };
        Some(next.format("%Y-%m-%d").to_string())
    }
}

impl Store {
    pub fn create_area(&self, p: &Value, actor: &str) -> Result<Value> {
        let name = require(p, "name")?;
        if name.chars().count() > 100 {
            return Err(LifeError::Invalid("Area name is too long".into()));
        }
        let color = optional(p, "color").unwrap_or_else(|| "#266C56".into());
        let icon = optional(p, "icon").unwrap_or_else(|| "layers".into());
        let key = id();
        self.conn.execute(
            "INSERT INTO areas(id,name,color,icon,created_at,archived) VALUES(?1,?2,?3,?4,?5,0)",
            params![key, name, color, icon, now()],
        )?;
        self.record(actor, "areas.create", Some(&key), p)?;
        self.fetch("areas", &key)
    }
    pub fn update_area(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let old = self.fetch("areas", &key)?;
        let name = optional(p, "name").unwrap_or_else(|| text(&old, "name"));
        if name.chars().count() > 100 {
            return Err(LifeError::Invalid("Area name is too long".into()));
        }
        let color = optional(p, "color").unwrap_or_else(|| text(&old, "color"));
        let icon = optional(p, "icon").unwrap_or_else(|| text(&old, "icon"));
        let archived = p
            .get("archived")
            .and_then(Value::as_bool)
            .map(i64::from)
            .unwrap_or_else(|| old.get("archived").and_then(Value::as_i64).unwrap_or(0));
        self.conn.execute(
            "UPDATE areas SET name=?2,color=?3,icon=?4,archived=?5 WHERE id=?1",
            params![key, name, color, icon, archived],
        )?;
        self.record(actor, "areas.update", Some(&key), p)?;
        self.fetch("areas", &key)
    }
    pub fn archive_area(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        self.fetch("areas", &key)?;
        self.conn
            .execute("UPDATE areas SET archived=1 WHERE id=?1", [&key])?;
        self.record(actor, "areas.delete", Some(&key), p)?;
        Ok(json!({"id":key,"archived":true,"reversible":true}))
    }
    pub fn restore_task(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let changed=self.conn.execute("UPDATE tasks SET deleted_at=NULL,revision=revision+1,updated_at=?2 WHERE id=?1 AND deleted_at IS NOT NULL",params![key,now()])?;
        if changed == 0 {
            return Err(LifeError::NotFound("No deleted task with that id".into()));
        }
        self.record(actor, "tasks.restore", Some(&key), p)?;
        self.fetch("tasks", &key)
    }
    pub fn verify_backup(&self, p: &Value) -> Result<Value> {
        use rusqlite::OpenFlags;
        let path = PathBuf::from(require(p, "path")?);
        if !path.is_file() {
            return Err(LifeError::NotFound("Backup file not found".into()));
        }
        // An encrypted backup is verified with its key. A backup without a
        // protection state file is read as a plain database.
        let state = super::encryption::read_state(&path)?;
        let conn = if state.protection == super::encryption::Protection::None {
            Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)?
        } else {
            let passphrase = p.get("passphrase").and_then(Value::as_str);
            let key =
                super::encryption::resolve_source_key(&state, self.key.as_deref(), passphrase)?;
            let connection = super::encryption::open_encrypted(&path, &key)?;
            connection.execute_batch("PRAGMA cipher_page_size = 4096; PRAGMA cipher_hmac_algorithm = HMAC_SHA512; PRAGMA cipher_kdf_algorithm = PBKDF2_HMAC_SHA512;")?;
            connection
        };
        let result: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if result != "ok" {
            return Err(LifeError::Conflict(format!(
                "Backup integrity failed: {result}"
            )));
        }
        let version: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        if version > super::SCHEMA_VERSION {
            return Err(LifeError::Conflict(format!(
                "Backup schema version {version} is newer than this app"
            )));
        }
        Ok(json!({
            "path":path.to_string_lossy(),
            "verified":true,
            "schema_version":version,
            "bytes":fs::metadata(path)?.len(),
            "encrypted":state.protection != super::encryption::Protection::None,
            "protection":state.protection.label()
        }))
    }
    pub fn restore_backup(&mut self, p: &Value, actor: &str) -> Result<Value> {
        use rusqlite::OpenFlags;
        let source = PathBuf::from(require(p, "path")?);
        if !p.get("confirm").and_then(Value::as_bool).unwrap_or(false) {
            return Err(LifeError::Invalid(
                "Set confirm=true after reviewing the backup and overwrite consequences".into(),
            ));
        }
        if fs::canonicalize(&source).ok() == fs::canonicalize(&self.path).ok() {
            return Err(LifeError::Invalid(
                "Cannot restore the current database over itself".into(),
            ));
        }
        self.verify_backup(p)?;
        let before = self.backup(&json!({}))?;
        // A plain backup is restored with the online backup API. An encrypted
        // backup is exported into the live database with the SQLCipher export
        // path, because the backup API does not work on encrypted databases.
        let source_state = super::encryption::read_state(&source)?;
        let live_state = super::encryption::read_state(&self.path)?;
        if source_state.protection == super::encryption::Protection::None
            && live_state.protection == super::encryption::Protection::None
        {
            let from = Connection::open_with_flags(&source, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            let backup = rusqlite::backup::Backup::new(&from, &mut self.conn)?;
            backup.run_to_completion(32, StdDuration::from_millis(20), None)?;
        } else if source_state.protection != super::encryption::Protection::None
            && live_state.protection == super::encryption::Protection::None
        {
            // The live database is plain but the backup is encrypted. The
            // backup is first exported into a plain copy, and that copy is
            // then restored into the live database with the backup API, so
            // the live database stays a plain database.
            let passphrase = p.get("passphrase").and_then(Value::as_str);
            let key = super::encryption::resolve_source_key(
                &source_state,
                self.key.as_deref(),
                passphrase,
            )?;
            let plain_copy = self.path.with_extension("restore-source.sqlite");
            let _ = fs::remove_file(&plain_copy);
            {
                let source = super::encryption::open_encrypted(&source, &key)?;
                source.execute_batch(&format!(
                    "ATTACH DATABASE '{}' AS plain KEY ''; \
                     SELECT sqlcipher_export('plain'); \
                     DETACH DATABASE plain;",
                    plain_copy.to_string_lossy()
                ))?;
            }
            let from = Connection::open_with_flags(&plain_copy, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            let backup = rusqlite::backup::Backup::new(&from, &mut self.conn)?;
            backup.run_to_completion(32, StdDuration::from_millis(20), None)?;
            let _ = fs::remove_file(&plain_copy);
        } else {
            // Both databases are encrypted. The backup is exported into a
            // fresh encrypted file, and that file replaces the live database,
            // so the protection stays the same.
            let passphrase = p.get("passphrase").and_then(Value::as_str);
            let key = super::encryption::resolve_source_key(
                &source_state,
                self.key.as_deref(),
                passphrase,
            )?;
            let encrypted_copy = self.path.with_extension("restore-source.sqlite");
            let _ = fs::remove_file(&encrypted_copy);
            {
                let source = super::encryption::open_encrypted(&source, &key)?;
                source.execute_batch(&format!(
                    "ATTACH DATABASE '{}' AS restored KEY \"x'{key}'\"; \
                     SELECT sqlcipher_export('restored'); \
                     DETACH DATABASE restored;",
                    encrypted_copy.to_string_lossy()
                ))?;
            }
            self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
            self.conn = Connection::open_in_memory()?;
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
            fs::rename(&encrypted_copy, &self.path)?;
            let reopened = super::encryption::open_encrypted(&self.path, &key)?;
            reopened.execute_batch(
                "PRAGMA cipher_page_size = 4096; PRAGMA cipher_hmac_algorithm = HMAC_SHA512; PRAGMA cipher_kdf_algorithm = PBKDF2_HMAC_SHA512; PRAGMA journal_mode=WAL;PRAGMA foreign_keys=ON;PRAGMA synchronous=NORMAL;",
            )?;
            self.swap_connection(reopened, Some(key))?;
        }
        self.conn.execute_batch("PRAGMA foreign_keys=ON")?;
        let result: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if result != "ok" {
            return Err(LifeError::Conflict(format!(
                "Restored database integrity failed: {result}; pre-restore copy at {}",
                before["path"]
            )));
        }
        self.migrate()?;
        self.record(
            actor,
            "backup.restore",
            None,
            &json!({"source":source.to_string_lossy(),"pre_restore_backup":before["path"]}),
        )?;
        Ok(
            json!({"restored":true,"source":source.to_string_lossy(),"pre_restore_backup":before["path"],"integrity":"ok"}),
        )
    }
}
