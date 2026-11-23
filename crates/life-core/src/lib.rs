//! Shared local-first domain for the desktop, CLI and MCP interfaces.
use chrono::{DateTime, Duration, Local, NaiveDate, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration as StdDuration,
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum LifeError {
    #[error("{0}")]
    Invalid(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, LifeError>;
/// The current SQLite schema version. Every migration block is guarded by
/// this value, so it is the single truth for the schema.
pub const SCHEMA_VERSION: i64 = 16;
pub fn error_code(e: &LifeError) -> &'static str {
    match e {
        LifeError::Invalid(_) => "VALIDATION_ERROR",
        LifeError::NotFound(_) => "NOT_FOUND",
        LifeError::Conflict(_) => "CONFLICT",
        LifeError::Sql(rusqlite::Error::SqliteFailure(_, _)) => "DATABASE_ERROR",
        LifeError::Sql(_) => "DATABASE_ERROR",
        LifeError::Io(_) => "IO_ERROR",
        LifeError::Json(_) => "JSON_ERROR",
    }
}
fn id() -> String {
    Uuid::now_v7().to_string()
}
fn now() -> String {
    Utc::now().to_rfc3339()
}
fn text(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}
fn optional(v: &Value, k: &str) -> Option<String> {
    let x = text(v, k);
    if x.is_empty() { None } else { Some(x) }
}
fn integer(v: &Value, k: &str, default: i64) -> i64 {
    v.get(k).and_then(Value::as_i64).unwrap_or(default)
}
fn require(v: &Value, k: &str) -> Result<String> {
    let x = text(v, k);
    if x.is_empty() {
        Err(LifeError::Invalid(format!("{k} is required")))
    } else {
        Ok(x)
    }
}
pub fn default_db_path() -> PathBuf {
    if let Ok(p) = std::env::var("LIFE_DB") {
        return PathBuf::from(p);
    }
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("LifeManager")
        .join("life.sqlite")
}
pub struct Store {
    pub conn: Connection,
    pub path: PathBuf,
    /// The master key of an encrypted database, cached for this session so
    /// the key is derived once and never has to be stored anywhere else.
    pub key: Option<String>,
}
impl Store {
    /// Open the database.
    ///
    /// A plain SQLite database opens directly. An encrypted database is
    /// detected from its recorded state and opened with the master key, which
    /// comes from the environment or a passphrase.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?
        }
        let state = encryption::read_state(&path)?;
        let conn = if state.protection == encryption::Protection::None {
            let conn = Connection::open(&path)?;
            conn.execute_batch(
                "PRAGMA journal_mode=WAL;PRAGMA foreign_keys=ON;PRAGMA synchronous=NORMAL;",
            )?;
            conn
        } else {
            let key = encryption::resolve_key(&state)?;
            let conn = encryption::open_encrypted(&path, &key)?;
            // The cipher pragmas must come first, so the WAL and the pragmas
            // are applied to the already-unlocked connection.
            conn.execute_batch(
                "PRAGMA cipher_page_size = 4096; PRAGMA cipher_hmac_algorithm = HMAC_SHA512; PRAGMA cipher_kdf_algorithm = PBKDF2_HMAC_SHA512; PRAGMA journal_mode=WAL;PRAGMA foreign_keys=ON;PRAGMA synchronous=NORMAL;",
            )?;
            conn
        };
        conn.busy_timeout(StdDuration::from_secs(5))?;
        let key = if state.protection == encryption::Protection::None {
            None
        } else {
            Some(encryption::resolve_key_with(&state, None)?)
        };
        let mut s = Self { conn, path, key };
        s.migrate()?;
        Ok(s)
    }

    /// Replace the live connection with a new one, closing the old handle
    /// first so the database file can be replaced underneath the store.
    pub fn swap_connection(&mut self, conn: Connection, key: Option<String>) -> Result<()> {
        conn.busy_timeout(StdDuration::from_secs(5))?;
        self.conn = conn;
        self.key = key;
        Ok(())
    }

    /// The master key of this session, if the database is encrypted.
    pub fn master_key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    /// Open the database with an explicit master key.
    ///
    /// This is how a caller that already holds the key opens an encrypted
    /// database, without going through the environment.
    pub fn open_with_key(path: impl AsRef<Path>, key: &str) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?
        }
        let conn = encryption::open_encrypted(&path, &key.to_string())?;
        conn.execute_batch(
            "PRAGMA cipher_page_size = 4096; PRAGMA cipher_hmac_algorithm = HMAC_SHA512; PRAGMA cipher_kdf_algorithm = PBKDF2_HMAC_SHA512; PRAGMA journal_mode=WAL;PRAGMA foreign_keys=ON;PRAGMA synchronous=NORMAL;",
        )?;
        conn.busy_timeout(StdDuration::from_secs(5))?;
        let mut s = Self {
            conn,
            path,
            key: Some(key.to_string()),
        };
        s.migrate()?;
        Ok(s)
    }

    /// Open the database with a passphrase, deriving the key with Argon2id.
    pub fn open_with_passphrase(path: impl AsRef<Path>, passphrase: &str) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let state = encryption::read_state(&path)?;
        let salt = state
            .salt
            .clone()
            .ok_or_else(|| LifeError::Invalid("The encryption state file has no salt".into()))?;
        let params = state.params.clone().unwrap_or_default();
        let key = encryption::derive_key(passphrase, &salt, &params)?;
        Self::open_with_key(path, &key)
    }

    /// Open the database, or explain exactly what is missing.
    ///
    /// Used by the CLI and the desktop app so a missing key never produces a
    /// confusing SQLite error.
    pub fn open_with_diagnostics(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let state = encryption::read_state(&path)?;
        if state.protection != encryption::Protection::None
            && encryption::resolve_key(&state).is_err()
            && std::env::var("LIFE_DB_KEY").is_err()
            && std::env::var("LIFE_DB_PASSPHRASE").is_err()
        {
            return Err(LifeError::Invalid(format!(
                "This database is {}. Provide the key and try again.",
                state.protection.label()
            )));
        }
        Self::open(path)
    }
    #[allow(clippy::should_implement_trait)] // Fallible initialization cannot implement infallible Default.
    pub fn default() -> Result<Self> {
        Self::open(default_db_path())
    }
    fn migrate(&mut self) -> Result<()> {
        self.conn.execute_batch(r#"
  CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY,applied_at TEXT NOT NULL);
  CREATE TABLE IF NOT EXISTS areas(id TEXT PRIMARY KEY,name TEXT NOT NULL UNIQUE,color TEXT NOT NULL DEFAULT '#266C56',icon TEXT NOT NULL DEFAULT 'layers',created_at TEXT NOT NULL);
  CREATE TABLE IF NOT EXISTS projects(id TEXT PRIMARY KEY,area_id TEXT REFERENCES areas(id) ON DELETE SET NULL,name TEXT NOT NULL,description TEXT NOT NULL DEFAULT '',status TEXT NOT NULL DEFAULT 'active',created_at TEXT NOT NULL,updated_at TEXT NOT NULL);
  CREATE TABLE IF NOT EXISTS goals(id TEXT PRIMARY KEY,area_id TEXT REFERENCES areas(id) ON DELETE SET NULL,title TEXT NOT NULL,target REAL NOT NULL DEFAULT 100,progress REAL NOT NULL DEFAULT 0,status TEXT NOT NULL DEFAULT 'active',due_date TEXT,created_at TEXT NOT NULL);
  CREATE TABLE IF NOT EXISTS tasks(id TEXT PRIMARY KEY,title TEXT NOT NULL,description TEXT NOT NULL DEFAULT '',status TEXT NOT NULL DEFAULT 'todo',priority INTEGER NOT NULL DEFAULT 3 CHECK(priority BETWEEN 1 AND 4),project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,area_id TEXT REFERENCES areas(id) ON DELETE SET NULL,parent_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,goal_id TEXT REFERENCES goals(id) ON DELETE SET NULL,due_date TEXT,planned_date TEXT,estimate_minutes INTEGER NOT NULL DEFAULT 30,recurrence TEXT NOT NULL DEFAULT '',tags TEXT NOT NULL DEFAULT '[]',created_at TEXT NOT NULL,updated_at TEXT NOT NULL,completed_at TEXT,deleted_at TEXT,revision INTEGER NOT NULL DEFAULT 1);
  CREATE INDEX IF NOT EXISTS tasks_status_due ON tasks(status,due_date) WHERE deleted_at IS NULL;
  CREATE INDEX IF NOT EXISTS tasks_project ON tasks(project_id) WHERE deleted_at IS NULL;
  CREATE TABLE IF NOT EXISTS events(id TEXT PRIMARY KEY,title TEXT NOT NULL,description TEXT NOT NULL DEFAULT '',start_at TEXT NOT NULL,end_at TEXT NOT NULL,all_day INTEGER NOT NULL DEFAULT 0,task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,kind TEXT NOT NULL DEFAULT 'event',color TEXT NOT NULL DEFAULT '#266C56',recurrence TEXT NOT NULL DEFAULT '',locked INTEGER NOT NULL DEFAULT 0,created_at TEXT NOT NULL,updated_at TEXT NOT NULL);
  CREATE INDEX IF NOT EXISTS events_start ON events(start_at);
  CREATE TABLE IF NOT EXISTS habits(id TEXT PRIMARY KEY,title TEXT NOT NULL,frequency TEXT NOT NULL DEFAULT 'daily',target INTEGER NOT NULL DEFAULT 1,unit TEXT NOT NULL DEFAULT 'times',color TEXT NOT NULL DEFAULT '#266C56',created_at TEXT NOT NULL,archived INTEGER NOT NULL DEFAULT 0);
  CREATE TABLE IF NOT EXISTS habit_logs(id TEXT PRIMARY KEY,habit_id TEXT NOT NULL REFERENCES habits(id) ON DELETE CASCADE,day TEXT NOT NULL,value INTEGER NOT NULL DEFAULT 1,created_at TEXT NOT NULL,UNIQUE(habit_id,day));
  CREATE TABLE IF NOT EXISTS notes(id TEXT PRIMARY KEY,title TEXT NOT NULL,content TEXT NOT NULL DEFAULT '',area_id TEXT REFERENCES areas(id) ON DELETE SET NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL);
  CREATE TABLE IF NOT EXISTS focus_sessions(id TEXT PRIMARY KEY,task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,started_at TEXT NOT NULL,ended_at TEXT,status TEXT NOT NULL DEFAULT 'active',duration_seconds INTEGER NOT NULL DEFAULT 0,mode TEXT NOT NULL DEFAULT 'stopwatch');
  CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
  CREATE TABLE IF NOT EXISTS action_history(id TEXT PRIMARY KEY,actor TEXT NOT NULL,action TEXT NOT NULL,entity_id TEXT,payload TEXT NOT NULL,created_at TEXT NOT NULL);
  CREATE TABLE IF NOT EXISTS idempotency_keys(key TEXT PRIMARY KEY,action TEXT NOT NULL,result TEXT NOT NULL);
  CREATE VIRTUAL TABLE IF NOT EXISTS task_search USING fts5(id UNINDEXED,title,description);
  INSERT OR IGNORE INTO schema_migrations VALUES(1,strftime('%Y-%m-%dT%H:%M:%fZ','now'));
  "#)?;
        if self
            .conn
            .query_row("SELECT COUNT(*) FROM areas", [], |r| r.get::<_, i64>(0))?
            == 0
        {
            for (name, color, icon) in [
                ("Projects", "#266C56", "briefcase-business"),
                ("Learning", "#4E7DAD", "graduation-cap"),
                ("Personal", "#B77D55", "heart"),
                ("Health", "#3B9571", "activity"),
                ("Finance", "#AE8955", "wallet"),
                ("Career", "#7B75B0", "target"),
            ] {
                self.conn.execute("INSERT OR IGNORE INTO areas(id,name,color,icon,created_at) VALUES(?1,?2,?3,?4,?5)",params![id(),name,color,icon,now()])?;
            }
        }
        self.conn.execute_batch(r#"
  CREATE TABLE IF NOT EXISTS milestones(id TEXT PRIMARY KEY,project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,title TEXT NOT NULL,due_date TEXT,status TEXT NOT NULL DEFAULT 'todo',created_at TEXT NOT NULL);
  CREATE TABLE IF NOT EXISTS routines(id TEXT PRIMARY KEY,title TEXT NOT NULL,description TEXT NOT NULL DEFAULT '',created_at TEXT NOT NULL,archived INTEGER NOT NULL DEFAULT 0);
  CREATE TABLE IF NOT EXISTS routine_steps(id TEXT PRIMARY KEY,routine_id TEXT NOT NULL REFERENCES routines(id) ON DELETE CASCADE,title TEXT NOT NULL,position INTEGER NOT NULL DEFAULT 0,habit_id TEXT REFERENCES habits(id) ON DELETE SET NULL,duration_minutes INTEGER NOT NULL DEFAULT 0);
  CREATE TABLE IF NOT EXISTS reviews(id TEXT PRIMARY KEY,period TEXT NOT NULL,day TEXT NOT NULL,title TEXT NOT NULL,content TEXT NOT NULL DEFAULT '',created_at TEXT NOT NULL,updated_at TEXT NOT NULL);
  CREATE TABLE IF NOT EXISTS automation_rules(id TEXT PRIMARY KEY,title TEXT NOT NULL,trigger_kind TEXT NOT NULL DEFAULT 'manual',action TEXT NOT NULL,arguments TEXT NOT NULL DEFAULT '{}',enabled INTEGER NOT NULL DEFAULT 1,created_at TEXT NOT NULL,last_run TEXT);
  CREATE TABLE IF NOT EXISTS automation_runs(id TEXT PRIMARY KEY,rule_id TEXT NOT NULL REFERENCES automation_rules(id) ON DELETE CASCADE,status TEXT NOT NULL,started_at TEXT NOT NULL,output TEXT NOT NULL DEFAULT '{}');
  CREATE TABLE IF NOT EXISTS task_dependencies(task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,blocks_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,PRIMARY KEY(task_id,blocks_id),CHECK(task_id<>blocks_id));
  CREATE TABLE IF NOT EXISTS focus_pauses(id TEXT PRIMARY KEY,session_id TEXT NOT NULL REFERENCES focus_sessions(id) ON DELETE CASCADE,started_at TEXT NOT NULL,ended_at TEXT);
  CREATE TABLE IF NOT EXISTS calendar_sources(id TEXT PRIMARY KEY,name TEXT NOT NULL,kind TEXT NOT NULL DEFAULT 'local',read_only INTEGER NOT NULL DEFAULT 0,color TEXT NOT NULL DEFAULT '#266C56');
  CREATE INDEX IF NOT EXISTS milestones_project ON milestones(project_id);
  CREATE INDEX IF NOT EXISTS habit_logs_day ON habit_logs(day,habit_id);
  CREATE INDEX IF NOT EXISTS reviews_period ON reviews(period,day);
  INSERT OR IGNORE INTO schema_migrations VALUES(2,strftime('%Y-%m-%dT%H:%M:%fZ','now'));
  "#)?;
        let schema_version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        if schema_version < 3 {
            self.conn.execute_batch("ALTER TABLE idempotency_keys ADD COLUMN payload TEXT NOT NULL DEFAULT '{}'; INSERT OR IGNORE INTO schema_migrations VALUES(3,strftime('%Y-%m-%dT%H:%M:%fZ','now'));")?;
        }
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        if version < 4 {
            self.conn.execute_batch("ALTER TABLE areas ADD COLUMN archived INTEGER NOT NULL DEFAULT 0; INSERT INTO schema_migrations(version,applied_at) VALUES(4,strftime('%Y-%m-%dT%H:%M:%fZ','now'));")?;
        }
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        if version < 5 {
            self.conn.execute_batch(r#"
              CREATE TABLE IF NOT EXISTS task_templates(id TEXT PRIMARY KEY,name TEXT NOT NULL,title TEXT NOT NULL,description TEXT NOT NULL DEFAULT '',priority INTEGER NOT NULL CHECK(priority BETWEEN 1 AND 4),estimate_minutes INTEGER NOT NULL DEFAULT 30,archived INTEGER NOT NULL DEFAULT 0,created_at TEXT NOT NULL);
              CREATE TABLE IF NOT EXISTS saved_filters(id TEXT PRIMARY KEY,entity TEXT NOT NULL,name TEXT NOT NULL,query_json TEXT NOT NULL,created_at TEXT NOT NULL);
              CREATE TABLE IF NOT EXISTS key_results(id TEXT PRIMARY KEY,goal_id TEXT NOT NULL REFERENCES goals(id) ON DELETE CASCADE,title TEXT NOT NULL,unit TEXT NOT NULL,baseline REAL NOT NULL,target REAL NOT NULL,current REAL NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL);
              CREATE TABLE IF NOT EXISTS goal_progress_history(id TEXT PRIMARY KEY,goal_id TEXT NOT NULL REFERENCES goals(id) ON DELETE CASCADE,key_result_id TEXT REFERENCES key_results(id) ON DELETE SET NULL,previous REAL NOT NULL,current REAL NOT NULL,created_at TEXT NOT NULL);
              CREATE TABLE IF NOT EXISTS availability_rules(id TEXT PRIMARY KEY,weekday INTEGER NOT NULL CHECK(weekday BETWEEN 0 AND 6),start_time TEXT NOT NULL,end_time TEXT NOT NULL,kind TEXT NOT NULL DEFAULT 'available');
              CREATE TABLE IF NOT EXISTS entity_links(id TEXT PRIMARY KEY,source_kind TEXT NOT NULL,source_id TEXT NOT NULL,target_kind TEXT NOT NULL,target_id TEXT NOT NULL,relation TEXT NOT NULL,created_at TEXT NOT NULL,UNIQUE(source_kind,source_id,target_kind,target_id,relation));
              CREATE INDEX IF NOT EXISTS key_results_goal_idx ON key_results(goal_id);
              CREATE INDEX IF NOT EXISTS links_source_idx ON entity_links(source_kind,source_id);
              CREATE INDEX IF NOT EXISTS links_target_idx ON entity_links(target_kind,target_id);
              CREATE INDEX IF NOT EXISTS progress_history_goal_idx ON goal_progress_history(goal_id,created_at);
              INSERT INTO schema_migrations(version,applied_at) VALUES(5,strftime('%Y-%m-%dT%H:%M:%fZ','now'));
            "#)?;
        }
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        if version < 6 {
            self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS action_changes(id TEXT PRIMARY KEY,actor TEXT NOT NULL,action TEXT NOT NULL,entity_id TEXT NOT NULL,request_id TEXT,before_json TEXT,after_json TEXT,undoable INTEGER NOT NULL DEFAULT 0,undone_at TEXT,created_at TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS action_changes_entity_idx ON action_changes(entity_id,created_at);
            INSERT INTO schema_migrations(version,applied_at) VALUES(6,strftime('%Y-%m-%dT%H:%M:%fZ','now'));
           "#)?;
        }
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        if version < 7 {
            self.conn.execute_batch(r#"
                CREATE TABLE IF NOT EXISTS career_applications(
                    id TEXT PRIMARY KEY, employer TEXT NOT NULL, role TEXT NOT NULL,
                    location TEXT NOT NULL DEFAULT '', stage TEXT NOT NULL DEFAULT 'saved',
                    applied_on TEXT, next_action TEXT NOT NULL DEFAULT '',next_action_on TEXT,
                    interview_at TEXT, job_url TEXT NOT NULL DEFAULT '', notes TEXT NOT NULL DEFAULT '',
                    salary_min INTEGER,salary_max INTEGER,currency TEXT NOT NULL DEFAULT 'SGD',
                    created_at TEXT NOT NULL,updated_at TEXT NOT NULL,revision INTEGER NOT NULL DEFAULT 1,deleted_at TEXT
                );
                CREATE TABLE IF NOT EXISTS learning_tracks(
                    id TEXT PRIMARY KEY,title TEXT NOT NULL,provider TEXT NOT NULL DEFAULT '',
                    status TEXT NOT NULL DEFAULT 'planned',progress REAL NOT NULL DEFAULT 0,
                    target_hours REAL NOT NULL DEFAULT 0,due_date TEXT,notes TEXT NOT NULL DEFAULT '',
                    url TEXT NOT NULL DEFAULT '',created_at TEXT NOT NULL,updated_at TEXT NOT NULL,
                    revision INTEGER NOT NULL DEFAULT 1,deleted_at TEXT
                );
                CREATE TABLE IF NOT EXISTS learning_sessions(
                    id TEXT PRIMARY KEY,track_id TEXT NOT NULL REFERENCES learning_tracks(id) ON DELETE CASCADE,
                    day TEXT NOT NULL,minutes INTEGER NOT NULL CHECK(minutes BETWEEN 1 AND 1440),
                    note TEXT NOT NULL DEFAULT '',created_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS finance_commitments(
                    id TEXT PRIMARY KEY,title TEXT NOT NULL,amount_minor INTEGER NOT NULL DEFAULT 0,
                    currency TEXT NOT NULL DEFAULT 'SGD',due_date TEXT,status TEXT NOT NULL DEFAULT 'pending',
                    category TEXT NOT NULL DEFAULT 'general',notes TEXT NOT NULL DEFAULT '',
                    created_at TEXT NOT NULL,updated_at TEXT NOT NULL,
                    revision INTEGER NOT NULL DEFAULT 1,deleted_at TEXT
                );
                CREATE TABLE IF NOT EXISTS wellbeing_checkins(
                    id TEXT PRIMARY KEY,day TEXT NOT NULL UNIQUE,
                    energy INTEGER NOT NULL CHECK(energy BETWEEN 1 AND 5),
                    mood INTEGER NOT NULL CHECK(mood BETWEEN 1 AND 5),
                    sleep_hours REAL,note TEXT NOT NULL DEFAULT '',created_at TEXT NOT NULL,updated_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS career_stage_next ON career_applications(stage,next_action_on) WHERE deleted_at IS NULL;
                CREATE INDEX IF NOT EXISTS learning_status_idx ON learning_tracks(status,due_date) WHERE deleted_at IS NULL;
                CREATE INDEX IF NOT EXISTS learning_sessions_track ON learning_sessions(track_id,day);
                CREATE INDEX IF NOT EXISTS finance_due_idx ON finance_commitments(due_date,status) WHERE deleted_at IS NULL;
                INSERT INTO schema_migrations(version,applied_at) VALUES(7,strftime('%Y-%m-%dT%H:%M:%fZ','now'));
            "#)?;
        }
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        if version < 8 {
            self.conn.execute_batch(r#"
                CREATE TABLE IF NOT EXISTS managed_attachments(
                    id TEXT PRIMARY KEY, owner_kind TEXT NOT NULL CHECK(owner_kind IN ('notes','tasks','projects','goals')),
                    owner_id TEXT NOT NULL,filename TEXT NOT NULL,mime_type TEXT NOT NULL,
                    content BLOB NOT NULL,bytes INTEGER NOT NULL CHECK(bytes BETWEEN 1 AND 8388608),
                    sha256 TEXT NOT NULL,created_at TEXT NOT NULL,deleted_at TEXT
                );
                CREATE INDEX IF NOT EXISTS managed_attachments_owner ON managed_attachments(owner_kind,owner_id,created_at);
                CREATE INDEX IF NOT EXISTS managed_attachments_hash ON managed_attachments(sha256);
                INSERT INTO schema_migrations(version,applied_at) VALUES(8,strftime('%Y-%m-%dT%H:%M:%fZ','now'));
            "#)?;
        }
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        if version < 9 {
            self.conn.execute_batch(r#"
                ALTER TABLE events ADD COLUMN external_uid TEXT;
                ALTER TABLE events ADD COLUMN import_source TEXT NOT NULL DEFAULT '';
                ALTER TABLE events ADD COLUMN timezone_id TEXT NOT NULL DEFAULT 'local';
                CREATE UNIQUE INDEX IF NOT EXISTS event_external_uid ON events(import_source,external_uid) WHERE external_uid IS NOT NULL;
                INSERT INTO schema_migrations(version,applied_at) VALUES(9,strftime('%Y-%m-%dT%H:%M:%fZ','now'));
            "#)?;
        }
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        if version < 10 {
            self.conn.execute_batch(r#"
                CREATE TABLE IF NOT EXISTS reminder_delivery(
                    key TEXT PRIMARY KEY,
                    claimed_until TEXT,delivered_at TEXT,dismissed_at TEXT,snoozed_until TEXT,
                    updated_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS reminder_delivery_latest ON reminder_delivery(updated_at DESC);
                INSERT INTO schema_migrations(version,applied_at) VALUES(10,strftime('%Y-%m-%dT%H:%M:%fZ','now'));
            "#)?;
        }
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        if version < 11 {
            self.conn.execute_batch(r#"
                ALTER TABLE habits ADD COLUMN quota_period TEXT NOT NULL DEFAULT 'daily';
                ALTER TABLE habits ADD COLUMN weekly_quota INTEGER NOT NULL DEFAULT 1;
                ALTER TABLE habits ADD COLUMN rest_days TEXT NOT NULL DEFAULT '[]';
                ALTER TABLE habit_logs ADD COLUMN status TEXT NOT NULL DEFAULT 'done';
                ALTER TABLE habit_logs ADD COLUMN note TEXT NOT NULL DEFAULT '';
                INSERT INTO schema_migrations(version,applied_at) VALUES(11,strftime('%Y-%m-%dT%H:%M:%fZ','now'));
            "#)?;
        }
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        if version < 12 {
            self.conn.execute_batch(
                r#"
              CREATE TABLE IF NOT EXISTS project_templates(
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                snapshot_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                archived_at TEXT
              );
              CREATE INDEX IF NOT EXISTS project_templates_recent_idx
                ON project_templates(archived_at,updated_at);
              INSERT INTO schema_migrations(version,applied_at)
                VALUES(12,strftime('%Y-%m-%dT%H:%M:%fZ','now'));
            "#,
            )?;
        }
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        if version < 13 {
            self.conn.execute_batch(r#"
              CREATE TABLE IF NOT EXISTS guided_reviews(
                id TEXT PRIMARY KEY, kind TEXT NOT NULL, day TEXT NOT NULL,
                state TEXT NOT NULL DEFAULT 'draft',
                answers_json TEXT NOT NULL DEFAULT '{}',
                metrics_json TEXT NOT NULL DEFAULT '{}',
                canonical_review_id TEXT REFERENCES reviews(id),
                created_at TEXT NOT NULL,updated_at TEXT NOT NULL,
                UNIQUE(kind,day)
              );
              CREATE INDEX IF NOT EXISTS guided_reviews_recent_idx ON guided_reviews(day,updated_at);
              INSERT INTO schema_migrations(version,applied_at)
                VALUES(13,strftime('%Y-%m-%dT%H:%M:%fZ','now'));
            "#)?;
        }
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        if version < 14 {
            self.conn.execute_batch(
                r#"
              CREATE TABLE IF NOT EXISTS goal_cycles(
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL CHECK(kind IN ('year','quarter')),
                period_key TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                starts_at TEXT NOT NULL,
                ends_at TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(kind,period_key)
              );
              CREATE TABLE IF NOT EXISTS goal_cycle_items(
                cycle_id TEXT NOT NULL REFERENCES goal_cycles(id) ON DELETE CASCADE,
                goal_id TEXT NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
                rank INTEGER NOT NULL CHECK(rank BETWEEN 1 AND 3),
                PRIMARY KEY(cycle_id,goal_id),
                UNIQUE(cycle_id,rank)
              );
              CREATE INDEX IF NOT EXISTS goal_cycles_recent_idx ON goal_cycles(starts_at,status);
              INSERT INTO schema_migrations(version,applied_at)
                VALUES(14,strftime('%Y-%m-%dT%H:%M:%fZ','now'));
            "#,
            )?;
        }
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        if version < 15 {
            self.conn.execute_batch(
                r#"
              CREATE TABLE IF NOT EXISTS recurrence_exceptions(
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
                occurrence_date TEXT NOT NULL,
                action TEXT NOT NULL CHECK(action IN ('skip','reschedule')),
                moved_to TEXT,
                reason TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                UNIQUE(task_id,occurrence_date)
              );
              CREATE INDEX IF NOT EXISTS recurrence_exceptions_task_idx
                ON recurrence_exceptions(task_id,occurrence_date);
              CREATE TABLE IF NOT EXISTS protected_blocks(
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                weekday INTEGER NOT NULL CHECK(weekday BETWEEN 0 AND 6),
                start_time TEXT NOT NULL,
                end_time TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'protected',
                created_at TEXT NOT NULL
              );
              CREATE TABLE IF NOT EXISTS time_budgets(
                id TEXT PRIMARY KEY,
                scope_kind TEXT NOT NULL CHECK(scope_kind IN ('area','project','global')),
                scope_id TEXT,
                period TEXT NOT NULL DEFAULT 'week',
                minutes INTEGER NOT NULL CHECK(minutes BETWEEN 0 AND 100000),
                created_at TEXT NOT NULL,
                UNIQUE(scope_kind,scope_id,period)
              );
              CREATE TABLE IF NOT EXISTS focus_presets(
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                focus_minutes INTEGER NOT NULL CHECK(focus_minutes BETWEEN 1 AND 480),
                break_minutes INTEGER NOT NULL DEFAULT 5 CHECK(break_minutes BETWEEN 0 AND 120),
                cycles INTEGER NOT NULL DEFAULT 4 CHECK(cycles BETWEEN 1 AND 12),
                created_at TEXT NOT NULL,
                UNIQUE(name)
              );
              CREATE TABLE IF NOT EXISTS saved_searches(
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                entity TEXT NOT NULL DEFAULT 'all',
                query_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(name)
              );
              CREATE TABLE IF NOT EXISTS agent_permissions(
                actor TEXT NOT NULL,
                action_pattern TEXT NOT NULL,
                allowed INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                PRIMARY KEY(actor,action_pattern)
              );
              CREATE TABLE IF NOT EXISTS agent_proposals(
                id TEXT PRIMARY KEY,
                actor TEXT NOT NULL,
                action TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                state TEXT NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','approved','rejected')),
                decision_reason TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                decided_at TEXT
              );
              CREATE INDEX IF NOT EXISTS agent_proposals_state_idx
                ON agent_proposals(state,created_at);
              CREATE TABLE IF NOT EXISTS automation_schedules(
                id TEXT PRIMARY KEY,
                rule_id TEXT NOT NULL REFERENCES automation_rules(id) ON DELETE CASCADE,
                kind TEXT NOT NULL CHECK(kind IN ('daily','weekly')),
                at_time TEXT NOT NULL,
                weekday INTEGER CHECK(weekday BETWEEN 0 AND 6),
                enabled INTEGER NOT NULL DEFAULT 1,
                last_run TEXT,
                created_at TEXT NOT NULL
              );
              ALTER TABLE focus_sessions ADD COLUMN preset_id TEXT;
              ALTER TABLE focus_sessions ADD COLUMN interruption_reason TEXT NOT NULL DEFAULT '';
              ALTER TABLE focus_sessions ADD COLUMN manual_minutes INTEGER NOT NULL DEFAULT 0;
              INSERT INTO schema_migrations(version,applied_at)
                VALUES(15,strftime('%Y-%m-%dT%H:%M:%fZ','now'));
            "#,
            )?;
        }
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        if version < 16 {
            self.conn.execute_batch(
                r#"
              CREATE TABLE IF NOT EXISTS interview_events(
                id TEXT PRIMARY KEY,
                application_id TEXT REFERENCES career_applications(id) ON DELETE SET NULL,
                title TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'interview',
                starts_at TEXT NOT NULL,
                ends_at TEXT,
                location TEXT NOT NULL DEFAULT '',
                preparation TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'scheduled',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                revision INTEGER NOT NULL DEFAULT 1,
                deleted_at TEXT
              );
              CREATE INDEX IF NOT EXISTS interview_events_start_idx ON interview_events(starts_at,status);
              CREATE TABLE IF NOT EXISTS savings_goals(
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                target_minor INTEGER NOT NULL DEFAULT 0,
                saved_minor INTEGER NOT NULL DEFAULT 0,
                currency TEXT NOT NULL DEFAULT 'SGD',
                due_date TEXT,
                note TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                revision INTEGER NOT NULL DEFAULT 1,
                deleted_at TEXT
              );
              CREATE TABLE IF NOT EXISTS certifications(
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                provider TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'planned',
                exam_date TEXT,
                deadline_date TEXT,
                study_hours_target REAL NOT NULL DEFAULT 0,
                study_hours_done REAL NOT NULL DEFAULT 0,
                note TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                revision INTEGER NOT NULL DEFAULT 1,
                deleted_at TEXT
              );
              CREATE TABLE IF NOT EXISTS rest_intentions(
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'sleep',
                starts_at TEXT NOT NULL DEFAULT '',
                ends_at TEXT NOT NULL DEFAULT '',
                days TEXT NOT NULL DEFAULT '[]',
                note TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                revision INTEGER NOT NULL DEFAULT 1,
                deleted_at TEXT
              );
              CREATE TABLE IF NOT EXISTS administration_items(
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'paperwork',
                renewal_date TEXT,
                reminder_days INTEGER NOT NULL DEFAULT 14,
                amount_minor INTEGER,
                currency TEXT NOT NULL DEFAULT 'SGD',
                status TEXT NOT NULL DEFAULT 'open',
                note TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                revision INTEGER NOT NULL DEFAULT 1,
                deleted_at TEXT
              );
              CREATE TABLE IF NOT EXISTS keyboard_shortcuts(
                id TEXT PRIMARY KEY,
                action TEXT NOT NULL UNIQUE,
                keys TEXT NOT NULL,
                label TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL
              );
              CREATE TABLE IF NOT EXISTS security_audit(
                id TEXT PRIMARY KEY,
                event TEXT NOT NULL,
                actor TEXT NOT NULL,
                detail TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL
              );
              CREATE INDEX IF NOT EXISTS security_audit_created_idx ON security_audit(created_at DESC);
              CREATE TABLE IF NOT EXISTS integration_accounts(
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                account_label TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'disconnected',
                scopes TEXT NOT NULL DEFAULT '[]',
                last_sync_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(provider,account_label)
              );
              CREATE TABLE IF NOT EXISTS remote_mcp_clients(
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                token_hash TEXT NOT NULL,
                scopes TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                last_used_at TEXT,
                revoked_at TEXT
              );
              CREATE TABLE IF NOT EXISTS notification_authorization(
                id TEXT PRIMARY KEY,
                bundle_id TEXT NOT NULL UNIQUE,
                status TEXT NOT NULL DEFAULT 'unknown',
                checked_at TEXT NOT NULL
              );
              CREATE TABLE IF NOT EXISTS imported_records(
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL,
                fingerprint TEXT NOT NULL,
                entity_kind TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                imported_at TEXT NOT NULL,
                UNIQUE(source,fingerprint)
              );
              INSERT INTO schema_migrations(version,applied_at)
                VALUES(16,strftime('%Y-%m-%dT%H:%M:%fZ','now'));
            "#,
            )?;
        }
        Ok(())
    }
    fn rows(&self, sql: &str, values: &[&dyn rusqlite::ToSql]) -> Result<Value> {
        let mut stmt = self.conn.prepare(sql)?;
        let names: Vec<String> = stmt.column_names().iter().map(|x| x.to_string()).collect();
        let r = stmt.query_map(values, |row| {
            let mut m = serde_json::Map::new();
            for (i, name) in names.iter().enumerate() {
                let raw = row.get_ref(i)?;
                let value = match raw {
                    rusqlite::types::ValueRef::Null => Value::Null,
                    rusqlite::types::ValueRef::Integer(v) => json!(v),
                    rusqlite::types::ValueRef::Real(v) => json!(v),
                    rusqlite::types::ValueRef::Text(v) => {
                        Value::String(String::from_utf8_lossy(v).into_owned())
                    }
                    rusqlite::types::ValueRef::Blob(_) => Value::Null,
                };
                m.insert(name.clone(), value);
            }
            Ok(Value::Object(m))
        })?;
        let all = r.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(json!(all))
    }
    fn record(
        &self,
        actor: &str,
        action: &str,
        entity_id: Option<&str>,
        payload: &Value,
    ) -> Result<()> {
        self.conn.execute("INSERT INTO action_history(id,actor,action,entity_id,payload,created_at) VALUES(?1,?2,?3,?4,?5,?6)",params![id(),actor,action,entity_id,payload.to_string(),now()])?;
        Ok(())
    }
    fn fetch(&self, table: &str, key: &str) -> Result<Value> {
        let allowed = [
            "tasks",
            "projects",
            "goals",
            "events",
            "habits",
            "notes",
            "areas",
            "focus_sessions",
        ];
        if !allowed.contains(&table) {
            return Err(LifeError::Invalid("Unsupported resource".into()));
        }
        let sql = format!(
            "SELECT * FROM {table} WHERE id=?1{} LIMIT 1",
            if table == "tasks" {
                " AND deleted_at IS NULL"
            } else {
                ""
            }
        );
        let mut values = self
            .rows(&sql, &[&key])?
            .as_array()
            .cloned()
            .unwrap_or_default();
        values
            .pop()
            .ok_or_else(|| LifeError::NotFound(format!("{table}:{key}")))
    }
    pub fn call(&mut self, action: &str, payload: &Value) -> Result<Value> {
        let actor = optional(payload, "actor").unwrap_or_else(|| "user".into());
        let request_id = optional(payload, "request_id");
        if !self.actor_permitted(&actor, action)? {
            return Err(LifeError::Conflict(format!(
                "Actor {actor} is not permitted to run {action}"
            )));
        }
        let write = action != "backup.create"
            && (action.ends_with(".create")
                || action.ends_with(".update")
                || action.ends_with(".delete")
                || action.ends_with(".complete")
                || action.ends_with(".log")
                || action.ends_with(".start")
                || action.ends_with(".stop")
                || action.ends_with(".pause")
                || action.ends_with(".resume")
                || action.ends_with(".add")
                || action.ends_with(".remove")
                || action.ends_with(".run")
                || action.ends_with(".apply")
                || action.ends_with(".replace")
                || action.ends_with(".bulk")
                || action.ends_with(".snooze")
                || action.ends_with(".archive")
                || action.ends_with(".capture")
                || action.ends_with(".prioritize")
                || action.ends_with(".configure")
                || action.ends_with(".skip")
                || action.ends_with(".claim")
                || action.ends_with(".delivered")
                || action.ends_with(".release")
                || action.ends_with(".dismiss")
                || action.ends_with(".undo")
                || action.ends_with(".restore")
                || action == "settings.set");
        if write && action != "backup.restore" {
            self.conn.execute_batch("BEGIN IMMEDIATE")?;
        }
        let result = (|| -> Result<Value> {
            if let Some(key) = &request_id
                && let Some((old_action, old_payload, old_result)) = self
                    .conn
                    .query_row(
                        "SELECT action,payload,result FROM idempotency_keys WHERE key=?1",
                        [key],
                        |r| {
                            Ok((
                                r.get::<_, String>(0)?,
                                r.get::<_, String>(1)?,
                                r.get::<_, String>(2)?,
                            ))
                        },
                    )
                    .optional()?
            {
                if old_action != action || serde_json::from_str::<Value>(&old_payload)? != *payload
                {
                    return Err(LifeError::Conflict(
                        "Idempotency key cannot be reused with different arguments".into(),
                    ));
                }
                return Ok(serde_json::from_str(&old_result)?);
            }
            let mut audit_key = optional(payload, "id");
            let audit_before = if ["tasks.update", "tasks.delete", "tasks.snooze"].contains(&action)
            {
                if let Some(key) = &audit_key {
                    self.task_snapshot(key)?
                } else {
                    None
                }
            } else {
                None
            };
            let output = self.dispatch(action, payload, &actor)?;
            if action == "tasks.create" {
                audit_key = optional(&output, "id");
            }
            if let Some(ref key) = audit_key
                && [
                    "tasks.create",
                    "tasks.update",
                    "tasks.delete",
                    "tasks.snooze",
                ]
                .contains(&action)
            {
                let audit_after = self.task_snapshot(key)?;
                self.write_change(
                    action,
                    &actor,
                    request_id.as_deref(),
                    key,
                    audit_before,
                    audit_after,
                )?;
            }

            if let Some(key) = request_id {
                self.conn.execute(
                    "INSERT INTO idempotency_keys(key,action,payload,result) VALUES(?1,?2,?3,?4)",
                    params![key, action, payload.to_string(), output.to_string()],
                )?;
            }
            Ok(output)
        })();
        if write && action != "backup.restore" {
            match result {
                Ok(value) => {
                    self.conn.execute_batch("COMMIT")?;
                    Ok(value)
                }
                Err(error) => {
                    let _ = self.conn.execute_batch("ROLLBACK");
                    Err(error)
                }
            }
        } else {
            result
        }
    }
    fn dispatch(&mut self, action: &str, p: &Value, actor: &str) -> Result<Value> {
        match action{
   "today"=>self.today(),"dashboard"=>self.dashboard(),
   "areas.list"=>self.rows("SELECT * FROM areas WHERE archived=0 ORDER BY name",&[]),
   "areas.create"=>self.create_area(p,actor),"areas.update"=>self.update_area(p,actor),"areas.delete"=>self.archive_area(p,actor),
   "projects.list"=>self.rows("SELECT p.*,a.name AS area FROM projects p LEFT JOIN areas a ON a.id=p.area_id ORDER BY p.updated_at DESC",&[]),
   "goals.list"=>self.rows("SELECT g.*,a.name AS area FROM goals g LEFT JOIN areas a ON a.id=g.area_id ORDER BY g.created_at DESC",&[]),
   "tasks.list"=>self.tasks_list(p),"tasks.get"=>self.fetch("tasks",&require(p,"id")?),
            "tasks.archived" => self.rows("SELECT * FROM tasks WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC LIMIT 250",&[]),
   "events.list"=>self.events_list(p),"habits.list"=>self.habits_list(),
   "notes.list"=>self.rows("SELECT * FROM notes ORDER BY updated_at DESC",&[]),
   "focus.list"=>self.rows("SELECT f.*,t.title AS task_title FROM focus_sessions f LEFT JOIN tasks t ON t.id=f.task_id ORDER BY f.started_at DESC LIMIT 100",&[]),
   "history.list"=>self.rows("SELECT * FROM action_history ORDER BY created_at DESC LIMIT 100",&[]),
   "settings.list"=>self.rows("SELECT key,value FROM settings ORDER BY key",&[]),
   "search"=>self.search(p),
   "stats"=>self.stats(),
   "projects.create"=>self.create_project(p,actor),"projects.update"=>self.update_project(p,actor),"projects.delete"=>self.delete("projects",p,actor),
   "goals.create"=>self.create_goal(p,actor),"goals.update"=>self.update_goal(p,actor),"goals.delete"=>self.delete("goals",p,actor),
   "tasks.create"=>self.create_task(p,actor),"tasks.update"=>self.update_task(p,actor),"tasks.complete"=>self.complete_task(p,actor),"tasks.delete"=>self.delete("tasks",p,actor),"tasks.restore"=>self.restore_task(p,actor),
   "events.create"=>self.create_event(p,actor),"events.update"=>self.update_event(p,actor),"events.delete"=>self.delete("events",p,actor),
   "habits.create"=>self.create_habit(p,actor),"habits.update"=>self.update_habit(p,actor),"habits.delete"=>self.delete("habits",p,actor),"habits.log"=>self.log_habit(p,actor),
   "habits.configure"=>self.habit_configure(p,actor),"habits.skip"=>self.habit_skip(p,actor),"habits.weekly"=>self.habit_weekly_progress(p),
   "notes.create"=>self.create_note(p,actor),"notes.update"=>self.update_note(p,actor),"notes.delete"=>self.delete("notes",p,actor),
   "focus.start"=>self.focus_start_with_mode(p,actor),"focus.stop"=>self.stop_focus(p,actor),
   "focus.pause"=>self.pause_focus(p,actor),"focus.resume"=>self.resume_focus(p,actor),
   "habits.history"=>self.habit_history(p),
   "routines.list"=>self.rows("SELECT * FROM routines WHERE archived=0 ORDER BY created_at DESC",&[]),
   "routines.create"=>self.create_routine(p,actor),"routines.update"=>self.update_routine(p,actor),"routines.steps"=>self.routine_steps(p),
   "routines.step.add"=>self.routine_step_add(p,actor),"routines.delete"=>self.delete_routine(p,actor),
   "milestones.list"=>self.milestones(p),"milestones.create"=>self.create_milestone(p,actor),"milestones.complete"=>self.complete_milestone(p,actor),
   "reviews.list"=>self.reviews(p),"reviews.create"=>self.create_review(p,actor),"reviews.update"=>self.update_review(p,actor),
   "dependencies.list"=>self.dependencies(p),"dependencies.add"=>self.add_dependency(p,actor),"dependencies.remove"=>self.remove_dependency(p,actor),
   "automations.list"=>self.rows("SELECT * FROM automation_rules ORDER BY created_at DESC",&[]),
   "automations.create"=>self.automation_create(p,actor),"automations.update"=>self.automation_update(p,actor),
   "automations.run"=>self.automation_run(p,actor),"automations.history"=>self.rows("SELECT * FROM automation_runs ORDER BY started_at DESC LIMIT 100",&[]),
   "settings.set"=>{let k=require(p,"key")?;let v=p.get("value").ok_or_else(||LifeError::Invalid("value required".into()))?;self.conn.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![k,v.to_string()])?;self.record(actor,action,None,p)?;Ok(json!({"key":k,"value":v}))},
   "calendar.conflicts"=>self.conflicts(p),
   "calendar.import.preview"=>self.ics_preview(p),
   "calendar.import.apply"=>self.ics_apply(p,actor),
   "reminders.settings"=>self.reminders_settings(),"reminders.upcoming"=>self.reminders_upcoming(p),"reminders.due"=>self.reminders_due(p),
   "reminders.claim"=>self.reminders_claim(p),"reminders.delivered"=>self.reminders_delivered(p,actor),"reminders.release"=>self.reminders_release(p),
   "reminders.dismiss"=>self.reminders_dismiss(p,actor),"reminders.snooze"=>self.reminders_snooze(p,actor),"reminders.history"=>self.reminders_history(),
   "schedule.propose"=>self.plan_capacity(p),
   "schedule.apply"=>self.apply_schedule(p,actor),
   "backup.create"=>self.backup(p),"backup.verify"=>self.verify_backup(p),"backup.restore"=>self.restore_backup(p,actor),
   "templates.list"=>self.templates_list(),"templates.create"=>self.templates_create(p,actor),"templates.delete"=>self.templates_archive(p,actor),"templates.apply"=>self.templates_apply(p,actor),
   "saved_views.list"=>self.saved_views_list(p),"saved_views.create"=>self.saved_views_create(p,actor),"saved_views.delete"=>self.saved_views_delete(p,actor),
   "key_results.list"=>self.key_results_list(p),"key_results.create"=>self.key_results_create(p,actor),"key_results.update"=>self.key_results_update(p,actor),"key_results.delete"=>self.key_results_delete(p,actor),"goals.history"=>self.goal_progress_history(p),
   "links.list"=>self.entity_links_list(p),"links.add"=>self.entity_links_add(p,actor),"links.remove"=>self.entity_links_remove(p,actor),
   "availability.list"=>self.availability_list(),"availability.replace"=>self.availability_replace(p,actor),
   "tasks.bulk"=>self.tasks_bulk(p,actor),"tasks.snooze"=>self.tasks_snooze(p,actor),
   "import.preview"=>self.import_preview(p),"import.apply"=>self.import_apply(p,actor),
   "reports.summary"=>self.reports_summary(p),"areas.dashboard"=>self.area_dashboard(p),"projects.timeline"=>self.project_timeline(p),"calendar.export_ics"=>self.calendar_export_ics(p),
   "agents.changes"=>self.agent_changes(),"agents.undo"=>self.agent_undo(p,actor),
   "attachments.list"=>self.attachments_list(p),"attachments.create"=>self.attachments_create(p,actor),"attachments.get"=>self.attachment_get(p),"attachments.delete"=>self.attachment_delete(p,actor),"attachments.restore"=>self.attachment_restore(p,actor),
   "career.list"=>self.domain_list("career",p),"career.get"=>self.domain_get("career",p),"career.create"=>self.domain_create("career",p,actor),"career.update"=>self.domain_update("career",p,actor),"career.delete"=>self.domain_delete("career",p,actor),"career.restore"=>self.domain_restore("career",p,actor),
   "learning.list"=>self.domain_list("learning",p),"learning.get"=>self.domain_get("learning",p),"learning.create"=>self.domain_create("learning",p,actor),"learning.update"=>self.domain_update("learning",p,actor),"learning.delete"=>self.domain_delete("learning",p,actor),"learning.restore"=>self.domain_restore("learning",p,actor),"learning.log"=>self.learning_log(p,actor),"learning.sessions"=>self.learning_sessions(p),
   "finance.list"=>self.domain_list("finance",p),"finance.get"=>self.domain_get("finance",p),"finance.create"=>self.domain_create("finance",p,actor),"finance.update"=>self.domain_update("finance",p,actor),"finance.delete"=>self.domain_delete("finance",p,actor),"finance.restore"=>self.domain_restore("finance",p,actor),
   "wellbeing.list"=>self.wellbeing_list(p),"wellbeing.log"=>self.wellbeing_log(p,actor),"wellbeing.delete"=>self.wellbeing_delete(p,actor),"life_domains.dashboard"=>self.life_domain_dashboard(),
   "projects.health"=>self.project_health(p),
   "projects.templates.list"=>self.project_templates_list(),
   "projects.templates.get"=>self.project_templates_get(p),
   "projects.template.capture"=>self.project_templates_capture(p,actor),
   "projects.template.apply"=>self.project_templates_apply(p,actor),
   "projects.template.archive"=>self.project_templates_archive(p,actor),
   "review_guides.prompts"=>self.review_guides_prompts(),
   "review_guides.list"=>self.review_guides_list(p),
   "review_guides.get"=>self.review_guides_get(p),
   "review_guides.start"=>self.review_guides_start(p,actor),
   "review_guides.update"=>self.review_guides_update(p,actor),
   "review_guides.complete"=>self.review_guides_complete(p,actor),
   "goal_cycles.list"=>self.goal_cycles_list(p),
   "goal_cycles.get"=>self.goal_cycles_get(p),
   "goal_cycles.create"=>self.goal_cycles_create(p,actor),
   "goal_cycles.update"=>self.goal_cycles_update(p,actor),
   "goal_cycles.prioritize"=>self.goal_cycles_prioritize(p,actor),
   "export"=>self.export(),
   "recurrence.exceptions.list"=>self.recurrence_exceptions_list(p),
   "recurrence.exceptions.create"=>self.recurrence_exceptions_create(p,actor),
   "recurrence.exceptions.delete"=>self.recurrence_exceptions_delete(p,actor),
   "recurrence.occurrences"=>self.recurrence_occurrences(p),
   "calendar.sources.list"=>self.calendar_sources_list(),
   "calendar.sources.create"=>self.calendar_sources_create(p,actor),
   "calendar.sources.update"=>self.calendar_sources_update(p,actor),
   "calendar.sources.delete"=>self.calendar_sources_delete(p,actor),
   "calendar.window"=>self.calendar_window(p),
   "protected.list"=>self.protected_blocks_list(p),
   "protected.create"=>self.protected_blocks_create(p,actor),
   "protected.delete"=>self.protected_blocks_delete(p,actor),
   "time_budgets.list"=>self.time_budgets_list(),
   "time_budgets.set"=>self.time_budgets_set(p,actor),
   "time_budgets.delete"=>self.time_budgets_delete(p,actor),
   "time_budgets.report"=>self.time_budget_report(p),
   "planning.carryover"=>self.planning_carryover(p),
   "planning.carryover.apply"=>self.planning_carryover_apply(p,actor),
   "planning.buffers"=>self.planning_buffers(p),
   "planning.next_week"=>self.planning_next_week(p),
   "focus.presets.list"=>self.focus_presets_list(),
   "focus.presets.create"=>self.focus_presets_create(p,actor),
   "focus.presets.delete"=>self.focus_presets_delete(p,actor),
   "focus.interrupt"=>self.focus_interrupt(p,actor),
   "focus.manual"=>self.focus_manual_entry(p,actor),
   "focus.trends"=>self.focus_trends(p),
   "saved_views.run"=>self.saved_views_run(p),
   "saved_searches.list"=>self.saved_searches_list(),
   "saved_searches.create"=>self.saved_searches_create(p,actor),
   "saved_searches.delete"=>self.saved_searches_delete(p,actor),
   "saved_searches.run"=>self.saved_searches_run(p),
   "links.backlinks"=>self.backlinks(p),
   "reports.plan_vs_actual"=>self.report_plan_vs_actual(p),
   "reports.goal_achievement"=>self.report_goal_achievement(p),
   "reports.area_distribution"=>self.report_area_distribution(p),
   "reports.journal_export"=>self.report_journal_export(p),
   "goals.limit"=>self.goal_limit_status(p),
   "accessibility.audit"=>self.accessibility_audit(p),
   "agents.permissions.list"=>self.agent_permissions_list(p),
   "agents.permissions.set"=>self.agent_permissions_set(p,actor),
   "agents.permissions.delete"=>self.agent_permissions_delete(p,actor),
   "agents.proposals.list"=>self.agent_proposals_list(p),
   "agents.proposals.create"=>self.agent_proposals_create(p,actor),
   "agents.proposals.approve"=>self.agent_proposals_approve(p,actor),
   "agents.proposals.reject"=>self.agent_proposals_reject(p,actor),
   "automations.schedule.list"=>self.automation_schedules_list(p),
   "automations.schedule.set"=>self.automation_schedules_set(p,actor),
   "automations.schedule.delete"=>self.automation_schedules_delete(p,actor),
   "automations.schedule.due"=>self.automation_schedules_due(p),
   "automations.schedule.run_due"=>self.automation_schedules_run_due(p,actor),
   "interviews.list"=>self.interviews_list(p),
   "interviews.create"=>self.interviews_create(p,actor),
   "interviews.update"=>self.interviews_update(p,actor),
   "interviews.delete"=>self.interviews_delete(p,actor),
   "savings_goals.list"=>self.savings_goals_list(),
   "savings_goals.create"=>self.savings_goals_create(p,actor),
   "savings_goals.update"=>self.savings_goals_update(p,actor),
   "savings_goals.delete"=>self.savings_goals_delete(p,actor),
   "certifications.list"=>self.certifications_list(),
   "certifications.create"=>self.certifications_create(p,actor),
   "certifications.update"=>self.certifications_update(p,actor),
   "certifications.delete"=>self.certifications_delete(p,actor),
   "rest_intentions.list"=>self.rest_intentions_list(),
   "rest_intentions.create"=>self.rest_intentions_create(p,actor),
   "rest_intentions.delete"=>self.rest_intentions_delete(p,actor),
   "administration.list"=>self.administration_list(),
   "administration.create"=>self.administration_create(p,actor),
   "administration.update"=>self.administration_update(p,actor),
   "administration.delete"=>self.administration_delete(p,actor),
   "personal.overview"=>self.personal_overview(),
   "entity.timeline"=>self.entity_timeline(p),
   "overdue.decisions"=>self.overdue_decisions(p),
   "overdue.decide"=>self.overdue_decide(p,actor),
   "projects.blockers"=>self.project_blockers(p),
   "projects.notes"=>self.project_notes(p),
   "projects.complete"=>self.project_complete(p,actor),
   "projects.reopen"=>self.project_reopen(p,actor),
   "goals.detail"=>self.goal_detail(p),
   "goals.monthly_review"=>self.goal_monthly_review(p),
   "reviews.journal_archive"=>self.journal_archive(p),
   "planning.session"=>self.planning_session(p),
   "planning.session.get"=>self.planning_session_get(p),
   "capture.parse"=>self.capture_parse(p),
   "system.status"=>self.system_status(),
   "encryption.status"=>self.encryption_status(),
   "encryption.enable"=>self.encryption_enable(p,actor),
   "encryption.disable"=>self.encryption_disable(p,actor),
   "encryption.verify_passphrase"=>self.encryption_verify_passphrase(p),
   "diagnostics.report"=>self.diagnostics_report(),
   "security.audit.log"=>self.security_audit_log(p,actor),
   "security.audit.list"=>self.security_audit_list(p),
   "csv.import.preview"=>self.csv_import_preview(p),
   "csv.import.apply"=>self.csv_import_apply(p,actor),
   "csv.import.history"=>self.csv_import_history(p),
   "shortcuts.list"=>self.shortcuts_list(),
   "shortcuts.update"=>self.shortcuts_update(p,actor),
   "shortcuts.reset"=>self.shortcuts_reset(p,actor),
   "integrations.list"=>self.integrations_list(),
   "integrations.configure"=>self.integrations_configure(p),
   "integrations.status"=>self.integrations_set_status(p,actor),
   "remote_clients.list"=>self.remote_clients_list(),
   "remote_clients.create"=>self.remote_clients_create(p,actor),
   "remote_clients.revoke"=>self.remote_clients_revoke(p,actor),
   "notification.authorization.set"=>self.notification_authorization_set(p,actor),
   "notification.authorization.get"=>self.notification_authorization_get(),
   _=>Err(LifeError::Invalid(format!("Unknown action: {action}")))
  }
    }
    fn tasks_list(&self, p: &Value) -> Result<Value> {
        let status = text(p, "status");
        let project = text(p, "project_id");
        let q = text(p, "query");
        let limit = integer(p, "limit", 250).clamp(1, 1000);
        let sql = "SELECT t.*,p.name AS project_name,a.name AS area_name FROM tasks t LEFT JOIN projects p ON t.project_id=p.id LEFT JOIN areas a ON t.area_id=a.id WHERE t.deleted_at IS NULL AND (?1='' OR t.status=?1) AND (?2='' OR t.project_id=?2) AND (?3='' OR t.title LIKE '%'||?3||'%' OR t.description LIKE '%'||?3||'%') ORDER BY CASE WHEN t.status='done' THEN 1 ELSE 0 END,t.priority ASC,COALESCE(t.due_date,'9999') ASC,t.created_at DESC LIMIT ?4";
        self.rows(sql, &[&status, &project, &q, &limit])
    }
    fn events_list(&self, p: &Value) -> Result<Value> {
        let from = text(p, "from");
        let to = text(p, "to");
        self.rows("SELECT e.*,t.title AS task_title FROM events e LEFT JOIN tasks t ON e.task_id=t.id WHERE (?1='' OR e.end_at>=?1) AND (?2='' OR e.start_at<=?2) ORDER BY start_at",&[&from,&to])
    }
    fn habits_list(&self) -> Result<Value> {
        self.rows("SELECT h.*,(SELECT COUNT(*) FROM habit_logs l WHERE l.habit_id=h.id AND l.value>0) AS completions,(SELECT value FROM habit_logs l WHERE l.habit_id=h.id AND l.day=date('now','localtime')) AS today_value FROM habits h WHERE archived=0 ORDER BY h.created_at DESC",&[])
    }
    fn today(&self) -> Result<Value> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let tasks=self.rows("SELECT * FROM tasks WHERE deleted_at IS NULL AND status NOT IN ('done','cancelled') AND (substr(COALESCE(due_date,''),1,10)<=?1 OR substr(COALESCE(planned_date,''),1,10)=?1) ORDER BY priority,due_date LIMIT 100",&[&today])?;
        let events=self.rows("SELECT * FROM events WHERE substr(start_at,1,10)<=?1 AND substr(end_at,1,10)>=?1 ORDER BY start_at LIMIT 100",&[&today])?;
        let done: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE status='done' AND substr(completed_at,1,10)=?1",
            [&today],
            |r| r.get(0),
        )?;
        let focus:i64=self.conn.query_row("SELECT COALESCE(SUM(duration_seconds),0) FROM focus_sessions WHERE substr(started_at,1,10)=?1 AND status='completed'",[&today],|r|r.get(0))?;
        Ok(
            json!({"date":today,"tasks":tasks,"events":events,"completed":done,"focus_seconds":focus}),
        )
    }
    fn dashboard(&self) -> Result<Value> {
        Ok(
            json!({"today":self.today()?,"stats":self.stats()?,"projects":self.dispatch_read_projects()?,"goals":self.rows("SELECT * FROM goals ORDER BY created_at DESC LIMIT 10",&[])?}),
        )
    }
    fn dispatch_read_projects(&self) -> Result<Value> {
        self.rows(
            "SELECT * FROM projects ORDER BY updated_at DESC LIMIT 12",
            &[],
        )
    }
    fn stats(&self) -> Result<Value> {
        let tasks:i64=self.conn.query_row("SELECT COUNT(*) FROM tasks WHERE deleted_at IS NULL AND status NOT IN ('done','cancelled')",[],|r|r.get(0))?;
        let done: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE deleted_at IS NULL AND status='done'",
            [],
            |r| r.get(0),
        )?;
        let projects: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM projects WHERE status='active'",
            [],
            |r| r.get(0),
        )?;
        let focus: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(duration_seconds),0) FROM focus_sessions WHERE status='completed'",
            [],
            |r| r.get(0),
        )?;
        let habits: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM habits WHERE archived=0", [], |r| {
                    r.get(0)
                })?;
        Ok(
            json!({"open_tasks":tasks,"completed_tasks":done,"active_projects":projects,"focus_seconds":focus,"habits":habits}),
        )
    }
    fn search(&self, p: &Value) -> Result<Value> {
        let q = require(p, "query")?;
        let t=self.rows("SELECT id,title,description,status,priority FROM tasks WHERE deleted_at IS NULL AND (title LIKE '%'||?1||'%' OR description LIKE '%'||?1||'%') ORDER BY updated_at DESC LIMIT 50",&[&q])?;
        let notes=self.rows("SELECT id,title,substr(content,1,240) AS content FROM notes WHERE title LIKE '%'||?1||'%' OR content LIKE '%'||?1||'%' LIMIT 50",&[&q])?;
        let projects=self.rows("SELECT id,name,description FROM projects WHERE name LIKE '%'||?1||'%' OR description LIKE '%'||?1||'%' LIMIT 30",&[&q])?;
        Ok(json!({"tasks":t,"notes":notes,"projects":projects}))
    }
    fn create_project(&self, p: &Value, actor: &str) -> Result<Value> {
        let name = require(p, "name")?;
        let key = id();
        let timestamp = now();
        let desc = text(p, "description");
        let area = optional(p, "area_id");
        self.conn.execute("INSERT INTO projects(id,area_id,name,description,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?5)",params![key,area,name,desc,timestamp])?;
        self.record(actor, "projects.create", Some(&key), p)?;
        self.fetch("projects", &key)
    }
    fn update_project(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        self.fetch("projects", &key)?;
        let current = self.fetch("projects", &key)?;
        let name = p
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(current["name"].as_str().unwrap_or(""));
        let description = p
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or(current["description"].as_str().unwrap_or(""));
        let status = p
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or(current["status"].as_str().unwrap_or("active"));
        let area = p
            .get("area_id")
            .and_then(Value::as_str)
            .or_else(|| current["area_id"].as_str());
        self.conn.execute("UPDATE projects SET name=?2,description=?3,status=?4,area_id=?5,updated_at=?6 WHERE id=?1",params![key,name,description,status,area,now()])?;
        self.record(actor, "projects.update", Some(&key), p)?;
        self.fetch("projects", &key)
    }
    fn create_goal(&self, p: &Value, actor: &str) -> Result<Value> {
        let title = require(p, "title")?;
        let key = id();
        let area = optional(p, "area_id");
        let target = p.get("target").and_then(Value::as_f64).unwrap_or(100.0);
        self.conn.execute("INSERT INTO goals(id,area_id,title,target,due_date,created_at) VALUES(?1,?2,?3,?4,?5,?6)",params![key,area,title,target,optional(p,"due_date"),now()])?;
        self.record(actor, "goals.create", Some(&key), p)?;
        self.fetch("goals", &key)
    }
    fn update_goal(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let current = self.fetch("goals", &key)?;
        let title = p
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or(current["title"].as_str().unwrap_or(""));
        let target = p
            .get("target")
            .and_then(Value::as_f64)
            .unwrap_or(current["target"].as_f64().unwrap_or(100.0));
        let progress = p
            .get("progress")
            .and_then(Value::as_f64)
            .unwrap_or(current["progress"].as_f64().unwrap_or(0.0));
        let status = p
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or(current["status"].as_str().unwrap_or("active"));
        self.conn.execute(
            "UPDATE goals SET title=?2,target=?3,progress=?4,status=?5 WHERE id=?1",
            params![key, title, target, progress, status],
        )?;
        self.record(actor, "goals.update", Some(&key), p)?;
        self.fetch("goals", &key)
    }
    fn create_task(&self, p: &Value, actor: &str) -> Result<Value> {
        let title = require(p, "title")?;
        let priority = integer(p, "priority", 3);
        if !(1..=4).contains(&priority) {
            return Err(LifeError::Invalid("priority must be 1..4".into()));
        }
        let key = id();
        let ts = now();
        let desc = text(p, "description");
        let status = optional(p, "status").unwrap_or_else(|| "todo".into());
        if ![
            "inbox",
            "todo",
            "in_progress",
            "blocked",
            "done",
            "cancelled",
        ]
        .contains(&status.as_str())
        {
            return Err(LifeError::Invalid("invalid task status".into()));
        }
        let tags = p.get("tags").cloned().unwrap_or_else(|| json!([]));
        let project = optional(p, "project_id");
        let area = optional(p, "area_id");
        let goal = optional(p, "goal_id");
        let parent = optional(p, "parent_id");
        let due = optional(p, "due_date");
        let planned = optional(p, "planned_date");
        let estimated = integer(p, "estimate_minutes", 30).clamp(0, 100_000);
        self.conn.execute("INSERT INTO tasks(id,title,description,status,priority,project_id,area_id,parent_id,goal_id,due_date,planned_date,estimate_minutes,recurrence,tags,created_at,updated_at,completed_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?15,?16)",params![key,title,desc,status,priority,project,area,parent,goal,due,planned,estimated,text(p,"recurrence"),tags.to_string(),ts,if status=="done"{Some(ts.clone())}else{None}])?;
        self.conn.execute(
            "INSERT INTO task_search(id,title,description) VALUES(?1,?2,?3)",
            params![key, title, desc],
        )?;
        self.record(actor, "tasks.create", Some(&key), p)?;
        self.fetch("tasks", &key)
    }
    fn update_task(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let current = self.fetch("tasks", &key)?;
        if let Some(expected) = p.get("expected_revision").and_then(Value::as_i64)
            && current["revision"].as_i64() != Some(expected)
        {
            return Err(LifeError::Conflict(
                "Task changed since your last read".into(),
            ));
        }
        let get = |k: &str| -> Value { p.get(k).cloned().unwrap_or_else(|| current[k].clone()) };
        let status = get("status").as_str().unwrap_or("todo").to_string();
        if ![
            "inbox",
            "todo",
            "in_progress",
            "blocked",
            "done",
            "cancelled",
        ]
        .contains(&status.as_str())
        {
            return Err(LifeError::Invalid("invalid status".into()));
        }
        let priority = get("priority").as_i64().unwrap_or(3);
        if !(1..=4).contains(&priority) {
            return Err(LifeError::Invalid("invalid priority".into()));
        }
        let ts = now();
        let completed = if status == "done" {
            current["completed_at"]
                .as_str()
                .map(str::to_string)
                .or_else(|| Some(ts.clone()))
        } else {
            None
        };
        let tags = p.get("tags").cloned().unwrap_or_else(|| {
            serde_json::from_str(current["tags"].as_str().unwrap_or("[]")).unwrap_or(json!([]))
        });
        self.conn.execute("UPDATE tasks SET title=?2,description=?3,status=?4,priority=?5,project_id=?6,area_id=?7,parent_id=?8,goal_id=?9,due_date=?10,planned_date=?11,estimate_minutes=?12,recurrence=?13,tags=?14,updated_at=?15,completed_at=?16,revision=revision+1 WHERE id=?1",params![key,get("title").as_str(),get("description").as_str(),status,priority,get("project_id").as_str(),get("area_id").as_str(),get("parent_id").as_str(),get("goal_id").as_str(),get("due_date").as_str(),get("planned_date").as_str(),get("estimate_minutes").as_i64(),get("recurrence").as_str(),tags.to_string(),ts,completed])?;
        self.conn
            .execute("DELETE FROM task_search WHERE id=?1", [&key])?;
        self.conn.execute(
            "INSERT INTO task_search(id,title,description) VALUES(?1,?2,?3)",
            params![key, get("title").as_str(), get("description").as_str()],
        )?;
        self.record(actor, "tasks.update", Some(&key), p)?;
        self.fetch("tasks", &key)
    }
    fn complete_task(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let previous = self.fetch("tasks", &key)?;
        if previous["status"] == "done" {
            return Ok(previous);
        }
        let mut copy = p.clone();
        copy["status"] = json!("done");
        let mut completed = self.update_task(&copy, actor)?;
        if let Some(next_date) = self.recurrence_next(&previous) {
            let next=self.create_task(&json!({
    "title":previous["title"],"description":previous["description"],"status":"todo",
    "priority":previous["priority"],"project_id":previous["project_id"],"area_id":previous["area_id"],
    "goal_id":previous["goal_id"],"parent_id":previous["parent_id"],"due_date":next_date,
    "estimate_minutes":previous["estimate_minutes"],"recurrence":previous["recurrence"],
    "tags":serde_json::from_str::<Value>(previous["tags"].as_str().unwrap_or("[]")).unwrap_or(json!([]))
   }),actor)?;
            completed["next_occurrence_id"] = next["id"].clone();
        }
        Ok(completed)
    }
    fn create_event(&self, p: &Value, actor: &str) -> Result<Value> {
        let title = require(p, "title")?;
        let start = require(p, "start_at")?;
        let end = require(p, "end_at")?;
        if end <= start {
            return Err(LifeError::Invalid(
                "end_at must be later than start_at".into(),
            ));
        }
        let key = id();
        let ts = now();
        let kind = optional(p, "kind").unwrap_or_else(|| "event".into());
        let locked = p.get("locked").and_then(Value::as_bool).unwrap_or(false);
        self.conn.execute("INSERT INTO events(id,title,description,start_at,end_at,all_day,task_id,kind,color,recurrence,locked,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?12)",params![key,title,text(p,"description"),start,end,p.get("all_day").and_then(Value::as_bool).unwrap_or(false),optional(p,"task_id"),kind,optional(p,"color").unwrap_or_else(||"#266C56".into()),text(p,"recurrence"),locked,ts])?;
        self.record(actor, "events.create", Some(&key), p)?;
        self.fetch("events", &key)
    }
    fn update_event(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let current = self.fetch("events", &key)?;
        let get = |k: &str| p.get(k).cloned().unwrap_or_else(|| current[k].clone());
        let start = get("start_at").as_str().unwrap_or("").to_owned();
        let end = get("end_at").as_str().unwrap_or("").to_owned();
        if end <= start {
            return Err(LifeError::Invalid("end_at must be after start_at".into()));
        }
        if current["locked"].as_i64() == Some(1)
            && (start != current["start_at"].as_str().unwrap_or("")
                || end != current["end_at"].as_str().unwrap_or(""))
            && !p
                .get("override_locked")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            return Err(LifeError::Conflict(
                "Locked event cannot be moved without override".into(),
            ));
        }
        self.conn.execute("UPDATE events SET title=?2,description=?3,start_at=?4,end_at=?5,all_day=?6,task_id=?7,kind=?8,color=?9,recurrence=?10,locked=?11,updated_at=?12 WHERE id=?1",params![key,get("title").as_str(),get("description").as_str(),start,end,get("all_day").as_i64(),get("task_id").as_str(),get("kind").as_str(),get("color").as_str(),get("recurrence").as_str(),get("locked").as_i64(),now()])?;
        self.record(actor, "events.update", Some(&key), p)?;
        self.fetch("events", &key)
    }
    fn create_habit(&self, p: &Value, actor: &str) -> Result<Value> {
        let title = require(p, "title")?;
        let key = id();
        self.conn.execute("INSERT INTO habits(id,title,frequency,target,unit,color,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![key,title,optional(p,"frequency").unwrap_or_else(||"daily".into()),integer(p,"target",1),optional(p,"unit").unwrap_or_else(||"times".into()),optional(p,"color").unwrap_or_else(||"#266C56".into()),now()])?;
        self.record(actor, "habits.create", Some(&key), p)?;
        self.fetch("habits", &key)
    }
    fn update_habit(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let current = self.fetch("habits", &key)?;
        let get = |k: &str| p.get(k).cloned().unwrap_or_else(|| current[k].clone());
        self.conn.execute("UPDATE habits SET title=?2,frequency=?3,target=?4,unit=?5,color=?6,archived=?7 WHERE id=?1",params![key,get("title").as_str(),get("frequency").as_str(),get("target").as_i64(),get("unit").as_str(),get("color").as_str(),get("archived").as_i64()])?;
        self.record(actor, "habits.update", Some(&key), p)?;
        self.fetch("habits", &key)
    }
    fn log_habit(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "habit_id")?;
        self.fetch("habits", &key)?;
        let day = optional(p, "day").unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
        NaiveDate::parse_from_str(&day, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("day must be YYYY-MM-DD".into()))?;
        let amount = integer(p, "value", 1);
        self.conn.execute("INSERT INTO habit_logs(id,habit_id,day,value,created_at,status,note) VALUES(?1,?2,?3,?4,?5,'done','') ON CONFLICT(habit_id,day) DO UPDATE SET value=excluded.value,status='done',note=''",params![id(),key,day,amount,now()])?;
        self.record(actor, "habits.log", Some(&key), p)?;
        Ok(json!({"habit_id":key,"day":day,"value":amount}))
    }
    fn create_note(&self, p: &Value, actor: &str) -> Result<Value> {
        let title = require(p, "title")?;
        let key = id();
        let ts = now();
        self.conn.execute("INSERT INTO notes(id,title,content,area_id,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?5)",params![key,title,text(p,"content"),optional(p,"area_id"),ts])?;
        self.record(actor, "notes.create", Some(&key), p)?;
        self.fetch("notes", &key)
    }
    fn update_note(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let current = self.fetch("notes", &key)?;
        let title = p
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or(current["title"].as_str().unwrap_or(""));
        let body = p
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or(current["content"].as_str().unwrap_or(""));
        self.conn.execute(
            "UPDATE notes SET title=?2,content=?3,updated_at=?4 WHERE id=?1",
            params![key, title, body, now()],
        )?;
        self.record(actor, "notes.update", Some(&key), p)?;
        self.fetch("notes", &key)
    }
    fn delete(&self, table: &str, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        self.fetch(table, &key)?;
        if table == "tasks" {
            self.conn.execute(
                "UPDATE tasks SET deleted_at=?2,revision=revision+1 WHERE id=?1",
                params![key, now()],
            )?;
            self.conn
                .execute("DELETE FROM task_search WHERE id=?1", [&key])?;
        } else if table == "habits" {
            self.conn
                .execute("UPDATE habits SET archived=1 WHERE id=?1", [&key])?;
        } else {
            let allowed = ["projects", "goals", "events", "notes"];
            if !allowed.contains(&table) {
                return Err(LifeError::Invalid("Unsupported resource deletion".into()));
            }
            self.conn
                .execute(&format!("DELETE FROM {table} WHERE id=?1"), [&key])?;
        }
        self.record(actor, &format!("{table}.delete"), Some(&key), p)?;
        Ok(json!({"deleted":true,"id":key}))
    }
    fn stop_focus(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let current = self.fetch("focus_sessions", &key)?;
        if current["status"] != "active" && current["status"] != "paused" {
            return Err(LifeError::Conflict("Session is no longer active".into()));
        }
        let from = DateTime::parse_from_rfc3339(current["started_at"].as_str().unwrap_or(""))
            .map_err(|_| LifeError::Invalid("Invalid session start".into()))?;
        let end = now();
        self.conn.execute(
            "UPDATE focus_pauses SET ended_at=?2 WHERE session_id=?1 AND ended_at IS NULL",
            params![key, end],
        )?;
        let pauses = self.rows(
            "SELECT started_at,ended_at FROM focus_pauses WHERE session_id=?1",
            &[&key],
        )?;
        let paused_seconds: i64 = pauses
            .as_array()
            .into_iter()
            .flatten()
            .map(|pause| {
                let started = pause["started_at"]
                    .as_str()
                    .and_then(|x| DateTime::parse_from_rfc3339(x).ok());
                let ended = pause["ended_at"]
                    .as_str()
                    .and_then(|x| DateTime::parse_from_rfc3339(x).ok());
                match (started, ended) {
                    (Some(a), Some(b)) => b.signed_duration_since(a).num_seconds().max(0),
                    _ => 0,
                }
            })
            .sum();
        let seconds = (Utc::now()
            .signed_duration_since(from.with_timezone(&Utc))
            .num_seconds()
            - paused_seconds)
            .max(0);
        self.conn.execute("UPDATE focus_sessions SET ended_at=?2,status='completed',duration_seconds=?3 WHERE id=?1",params![key,now(),seconds])?;
        self.record(actor, "focus.stop", Some(&key), p)?;
        self.fetch("focus_sessions", &key)
    }
    fn conflicts(&self, p: &Value) -> Result<Value> {
        let start = require(p, "start_at")?;
        let end = require(p, "end_at")?;
        if end <= start {
            return Err(LifeError::Invalid("Invalid interval".into()));
        }
        self.rows(
            "SELECT * FROM events WHERE start_at<?2 AND end_at>?1 ORDER BY start_at",
            &[&start, &end],
        )
    }
    fn backup(&self, p: &Value) -> Result<Value> {
        let dest = optional(p, "path").map(PathBuf::from).unwrap_or_else(|| {
            self.path.with_file_name(format!(
                "life-backup-{}-{}.sqlite",
                Utc::now().format("%Y%m%d-%H%M%S"),
                id()
            ))
        });
        if dest == self.path {
            return Err(LifeError::Invalid(
                "Backup path cannot equal database".into(),
            ));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?
        }
        if dest.exists() {
            return Err(LifeError::Conflict(
                "Backup destination already exists; choose a new path".into(),
            ));
        }
        // A plain database is copied with the online backup API. An
        // encrypted database cannot use that API, because SQLCipher replaces
        // the page format, so it is exported with the SQLCipher export path
        // into a new encrypted file.
        let state = encryption::read_state(&self.path)?;
        if state.protection == encryption::Protection::None {
            let mut other = Connection::open(&dest)?;
            let b = rusqlite::backup::Backup::new(&self.conn, &mut other)?;
            b.run_to_completion(32, StdDuration::from_millis(20), None)?;
            drop(b);
            other
                .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .and_then(|result| {
                    if result == "ok" {
                        Ok(())
                    } else {
                        Err(rusqlite::Error::InvalidQuery)
                    }
                })?;
        } else {
            let key = encryption::resolve_key_with(&state, self.key.as_deref())?;
            self.conn.execute_batch(&format!(
                "ATTACH DATABASE '{}' AS snapshot KEY \"x'{key}'\";                  SELECT sqlcipher_export('snapshot');                  DETACH DATABASE snapshot;",
                dest.to_string_lossy()
            ))?;
            let check = encryption::open_encrypted(&dest, &key)?;
            let integrity: String = check.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
            if integrity != "ok" {
                return Err(LifeError::Conflict(
                    "The encrypted backup failed its integrity check".into(),
                ));
            }
        }
        Ok(json!({
            "path":dest.to_string_lossy(),
            "verified":true,
            "bytes":fs::metadata(&dest)?.len(),
            "encrypted":state.protection != encryption::Protection::None,
            "protection":state.protection.label()
        }))
    }
    fn export(&self) -> Result<Value> {
        let mut out = serde_json::Map::new();
        for table in [
            "areas",
            "projects",
            "goals",
            "tasks",
            "events",
            "habits",
            "habit_logs",
            "notes",
            "focus_sessions",
            "settings",
        ] {
            let q = format!("SELECT * FROM {table}");
            out.insert(table.into(), self.rows(&q, &[])?);
        }
        out.insert("managed_attachments".into(),self.rows("SELECT id,owner_kind,owner_id,filename,mime_type,bytes,sha256,created_at,deleted_at FROM managed_attachments",&[])?);
        out.insert(
            "career_applications".into(),
            self.rows("SELECT * FROM career_applications", &[])?,
        );
        out.insert(
            "learning_tracks".into(),
            self.rows("SELECT * FROM learning_tracks", &[])?,
        );
        out.insert(
            "learning_sessions".into(),
            self.rows("SELECT * FROM learning_sessions", &[])?,
        );
        out.insert(
            "finance_commitments".into(),
            self.rows("SELECT * FROM finance_commitments", &[])?,
        );
        out.insert(
            "wellbeing_checkins".into(),
            self.rows("SELECT * FROM wellbeing_checkins", &[])?,
        );
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        out.insert(
            "reminder_delivery".into(),
            self.rows("SELECT * FROM reminder_delivery", &[])?,
        );
        out.insert("schema_version".into(), json!(version));
        out.insert("exported_at".into(), json!(now()));
        Ok(Value::Object(out))
    }
}
mod agent_audit;
mod agent_policy;
mod attachments;
#[cfg(test)]
mod attachments_tests;
mod calendar_export;
#[cfg(test)]
mod calendar_export_tests;
mod calendar_import;
#[cfg(test)]
mod calendar_import_tests;
mod calendar_rules;
mod capacity;
#[cfg(test)]
mod capacity_tests;
mod csv_import;
mod encryption;
mod encryption_ops;
#[cfg(test)]
mod encryption_tests;
#[cfg(test)]
mod expanded_tests;
mod extensions;
mod focus_modes;
mod goal_cycles;
#[cfg(test)]
mod goal_cycles_tests;
mod guided_reviews;
#[cfg(test)]
mod guided_reviews_tests;
mod habit_advanced;
#[cfg(test)]
mod habit_advanced_tests;
mod insight_reports;
mod life_area_extras;
mod life_domains;
#[cfg(test)]
mod life_domains_tests;
mod parity;
#[cfg(test)]
mod parity_tests;
mod planning_rules;
mod project_advanced;
#[cfg(test)]
mod project_advanced_tests;
mod reminders;
#[cfg(test)]
mod reminders_tests;
mod security_and_settings;
#[cfg(test)]
mod v15_tests;
#[cfg(test)]
mod v16_tests;
mod view_filters;
mod workflow_gaps;
#[cfg(test)]
mod tests {
    use super::*;
    fn store() -> Store {
        let conn = Connection::open_in_memory().unwrap();
        drop(conn);
        let path = std::env::temp_dir().join(format!("life-test-{}.sqlite", id()));
        Store::open(path).unwrap()
    }
    #[test]
    fn create_update_complete_and_delete_task() {
        let mut s = store();
        let task = s
            .call("tasks.create", &json!({"title":"Write test","priority":1}))
            .unwrap();
        assert_eq!(task["revision"], 1);
        let u = s
            .call(
                "tasks.update",
                &json!({"id":task["id"],"title":"Updated","expected_revision":1}),
            )
            .unwrap();
        assert_eq!(u["revision"], 2);
        assert!(
            s.call(
                "tasks.update",
                &json!({"id":task["id"],"expected_revision":1})
            )
            .is_err()
        );
        let c = s.call("tasks.complete", &json!({"id":task["id"]})).unwrap();
        assert_eq!(c["status"], "done");
        s.call("tasks.delete", &json!({"id":task["id"]})).unwrap();
        assert_eq!(
            s.call("tasks.list", &json!({}))
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }
    #[test]
    fn habit_logs_and_focus() {
        let mut s = store();
        let h = s
            .call("habits.create", &json!({"title":"Meditate"}))
            .unwrap();
        s.call(
            "habits.log",
            &json!({"habit_id":h["id"],"day":"2026-11-09","value":15}),
        )
        .unwrap();
        assert_eq!(
            s.call("habits.list", &json!({}))
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let session = s.call("focus.start", &json!({})).unwrap();
        assert!(s.call("focus.start", &json!({})).is_err());
        assert_eq!(
            s.call("focus.stop", &json!({"id":session["id"]})).unwrap()["status"],
            "completed"
        );
    }
    #[test]
    fn idempotency_and_calendar_conflict() {
        let mut s = store();
        let p = json!({"title":"One task","request_id":"a-request"});
        s.call("tasks.create", &p).unwrap();
        s.call("tasks.create", &p).unwrap();
        assert_eq!(
            s.call("tasks.list", &json!({}))
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            1
        );
        s.call("events.create",&json!({"title":"Meeting","start_at":"2026-11-10T10:00:00","end_at":"2026-11-10T11:00:00"})).unwrap();
        let c = s
            .call(
                "calendar.conflicts",
                &json!({"start_at":"2026-11-10T10:30:00","end_at":"2026-11-10T11:30:00"}),
            )
            .unwrap();
        assert_eq!(c.as_array().unwrap().len(), 1);
    }
    #[test]
    fn verified_backup() {
        let mut s = store();
        let result = s.call("backup.create", &json!({})).unwrap();
        assert_eq!(result["verified"], true);
    }
}
