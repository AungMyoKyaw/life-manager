//! Focus modes: pomodoro presets, manual time entries, interruption reasons
//! and focus trends computed from recorded sessions.
use super::*;

impl Store {
    pub fn focus_presets_list(&self) -> Result<Value> {
        self.rows("SELECT * FROM focus_presets ORDER BY name", &[])
    }

    pub fn focus_presets_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let name = require(p, "name")?;
        let focus_minutes = integer(p, "focus_minutes", 25);
        if !(1..=480).contains(&focus_minutes) {
            return Err(LifeError::Invalid(
                "focus_minutes must be 1 through 480".into(),
            ));
        }
        let break_minutes = integer(p, "break_minutes", 5);
        if !(0..=120).contains(&break_minutes) {
            return Err(LifeError::Invalid(
                "break_minutes must be 0 through 120".into(),
            ));
        }
        let cycles = integer(p, "cycles", 4);
        if !(1..=12).contains(&cycles) {
            return Err(LifeError::Invalid("cycles must be 1 through 12".into()));
        }
        let key = id();
        self.conn.execute(
            "INSERT INTO focus_presets(id,name,focus_minutes,break_minutes,cycles,created_at) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(name) DO UPDATE SET focus_minutes=excluded.focus_minutes,break_minutes=excluded.break_minutes,cycles=excluded.cycles",
            params![key, name, focus_minutes, break_minutes, cycles, now()],
        )?;
        self.record(actor, "focus.presets.create", Some(&key), p)?;
        self.rows("SELECT * FROM focus_presets WHERE name=?1", &[&name])
            .map(|v| v[0].clone())
    }

    pub fn focus_presets_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let n = self
            .conn
            .execute("DELETE FROM focus_presets WHERE id=?1", [&key])?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "focus.presets.delete", Some(&key), p)?;
        Ok(json!({"deleted":true,"id":key}))
    }

    /// Start a focus session. Mode is stopwatch, pomodoro or manual.
    /// A pomodoro session must name a preset.
    pub fn focus_start_with_mode(&self, p: &Value, actor: &str) -> Result<Value> {
        let mode = optional(p, "mode").unwrap_or_else(|| "stopwatch".into());
        if !["stopwatch", "pomodoro", "manual"].contains(&mode.as_str()) {
            return Err(LifeError::Invalid(
                "mode must be stopwatch, pomodoro or manual".into(),
            ));
        }
        let preset_id = optional(p, "preset_id");
        if mode == "pomodoro" {
            let preset = preset_id.clone().ok_or_else(|| {
                LifeError::Invalid("preset_id is required for a pomodoro session".into())
            })?;
            let found = self
                .rows("SELECT id FROM focus_presets WHERE id=?1", &[&preset])?
                .as_array()
                .map(|v| !v.is_empty())
                .unwrap_or(false);
            if !found {
                return Err(LifeError::NotFound(preset));
            }
        }
        let key = id();
        let task = optional(p, "task_id");
        if let Some(ref k) = task {
            self.fetch("tasks", k)?;
        }
        let active: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM focus_sessions WHERE status IN ('active','paused')",
            [],
            |r| r.get(0),
        )?;
        if active > 0 {
            return Err(LifeError::Conflict(
                "Another focus session is already active".into(),
            ));
        }
        self.conn.execute(
            "INSERT INTO focus_sessions(id,task_id,started_at,mode,preset_id) VALUES(?1,?2,?3,?4,?5)",
            params![key, task, now(), mode, preset_id],
        )?;
        self.record(actor, "focus.start", Some(&key), p)?;
        self.fetch("focus_sessions", &key)
    }

    /// Record an interruption reason on a session.
    pub fn focus_interrupt(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let reason = require(p, "reason")?;
        let current = self.fetch("focus_sessions", &key)?;
        if current["status"] != "active" && current["status"] != "paused" {
            return Err(LifeError::Conflict(
                "Only an active session can be interrupted".into(),
            ));
        }
        self.conn.execute(
            "UPDATE focus_sessions SET interruption_reason=?2,status='paused' WHERE id=?1",
            params![key, reason],
        )?;
        self.record(actor, "focus.interrupt", Some(&key), p)?;
        self.fetch("focus_sessions", &key)
    }

    /// Record a completed work session manually, for work that happened
    /// outside the timer. The session is stored with mode 'manual'.
    pub fn focus_manual_entry(&self, p: &Value, actor: &str) -> Result<Value> {
        let minutes = integer(p, "minutes", 0);
        if !(1..=1440).contains(&minutes) {
            return Err(LifeError::Invalid("minutes must be 1 through 1440".into()));
        }
        let task = optional(p, "task_id");
        if let Some(ref k) = task {
            self.fetch("tasks", k)?;
        }
        let day = optional(p, "day").unwrap_or_else(|| Local::now().date_naive().to_string());
        let started = format!("{day}T00:00:00");
        let key = id();
        self.conn.execute(
            "INSERT INTO focus_sessions(id,task_id,started_at,ended_at,status,duration_seconds,mode,manual_minutes,interruption_reason) VALUES(?1,?2,?3,?4,'completed',?5,'manual',?6,?7)",
            params![key, task, started, started, minutes * 60, minutes, text(p, "reason")],
        )?;
        self.record(actor, "focus.manual", Some(&key), p)?;
        self.fetch("focus_sessions", &key)
    }

    /// Focus trends over a window: recorded minutes per day, the planned
    /// estimate of the linked tasks, and the difference.
    pub fn focus_trends(&self, p: &Value) -> Result<Value> {
        let from = optional(p, "from").unwrap_or_else(|| {
            (Local::now().date_naive() - Duration::days(27))
                .format("%Y-%m-%d")
                .to_string()
        });
        let to = optional(p, "to").unwrap_or_else(|| Local::now().date_naive().to_string());
        let sessions = self.rows(
            "SELECT substr(started_at,1,10) AS day, COALESCE(SUM(duration_seconds),0) AS seconds, COUNT(*) AS sessions FROM focus_sessions WHERE status='completed' AND substr(started_at,1,10)>=?1 AND substr(started_at,1,10)<=?2 GROUP BY substr(started_at,1,10) ORDER BY day",
            &[&from, &to],
        )?;
        let planned = self.rows(
            "SELECT substr(planned_date,1,10) AS day, COALESCE(SUM(estimate_minutes),0) AS minutes FROM tasks WHERE deleted_at IS NULL AND planned_date>=?1 AND planned_date<=?2 GROUP BY substr(planned_date,1,10)",
            &[&from, &to],
        )?;
        let mut rows: Vec<Value> = Vec::new();
        let mut total_focused = 0i64;
        for session in sessions.as_array().into_iter().flatten() {
            let day = session["day"].as_str().unwrap_or("");
            let seconds = session["seconds"].as_i64().unwrap_or(0);
            total_focused += seconds;
            let planned_minutes = planned
                .as_array()
                .into_iter()
                .flatten()
                .find(|r| r["day"].as_str() == Some(day))
                .and_then(|r| r["minutes"].as_i64())
                .unwrap_or(0);
            rows.push(json!({
                "day":day,
                "focused_seconds":seconds,
                "focused_minutes":seconds / 60,
                "sessions":session["sessions"].as_i64().unwrap_or(0),
                "planned_minutes":planned_minutes,
                "difference_minutes":seconds / 60 - planned_minutes,
            }));
        }
        let interruptions = self.rows(
            "SELECT interruption_reason, COUNT(*) AS count FROM focus_sessions WHERE interruption_reason<>'' GROUP BY interruption_reason ORDER BY count DESC",
            &[],
        )?;
        Ok(json!({
            "from":from,
            "to":to,
            "days":rows,
            "total_focused_minutes":total_focused / 60,
            "interruptions":interruptions,
            "source":"Local SQLite: completed focus sessions and task estimates inside the window.",
        }))
    }
}
