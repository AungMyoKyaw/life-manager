//! Calendar rules: recurrence exceptions, occurrence expansion, calendar sources,
//! and all-day and time-zone aware event reads.
use super::*;
use chrono::{Datelike, Weekday};

impl Store {
    /// The next occurrence date of a recurring task, honouring any recorded
    /// exception for that exact date. Returns None when the task does not
    /// recur or when the next occurrence was skipped.
    pub fn recurrence_next_with_exceptions(&self, current: &Value) -> Option<String> {
        let base = self.recurrence_next(current)?;
        let task_id = current["id"].as_str().unwrap_or("");
        if task_id.is_empty() {
            return Some(base);
        }
        let exception = self
            .rows(
                "SELECT action,moved_to FROM recurrence_exceptions WHERE task_id=?1 AND occurrence_date=?2",
                &[&task_id, &base],
            )
            .ok()?;
        let row = exception.as_array().and_then(|v| v.first()).cloned();
        match row {
            None => Some(base),
            Some(row) => match row["action"].as_str().unwrap_or("skip") {
                "reschedule" => row["moved_to"].as_str().map(|v| v.to_string()),
                _ => None,
            },
        }
    }

    pub fn recurrence_exceptions_list(&self, p: &Value) -> Result<Value> {
        let task = require(p, "task_id")?;
        self.rows(
            "SELECT * FROM recurrence_exceptions WHERE task_id=?1 ORDER BY occurrence_date",
            &[&task],
        )
    }

    pub fn recurrence_exceptions_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let task = require(p, "task_id")?;
        let occurrence = require(p, "occurrence_date")?;
        NaiveDate::parse_from_str(&occurrence, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("occurrence_date must be YYYY-MM-DD".into()))?;
        let action = optional(p, "action").unwrap_or_else(|| "skip".into());
        if !["skip", "reschedule"].contains(&action.as_str()) {
            return Err(LifeError::Invalid(
                "action must be skip or reschedule".into(),
            ));
        }
        let moved_to = optional(p, "moved_to");
        if action == "reschedule" {
            let target = moved_to.clone().ok_or_else(|| {
                LifeError::Invalid("moved_to is required for a reschedule".into())
            })?;
            NaiveDate::parse_from_str(&target, "%Y-%m-%d")
                .map_err(|_| LifeError::Invalid("moved_to must be YYYY-MM-DD".into()))?;
        }
        let exists = self
            .rows("SELECT id FROM tasks WHERE id=?1", &[&task])?
            .as_array()
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        if !exists {
            return Err(LifeError::NotFound(task));
        }
        let key = id();
        self.conn.execute(
            "INSERT INTO recurrence_exceptions(id,task_id,occurrence_date,action,moved_to,reason,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(task_id,occurrence_date) DO UPDATE SET action=excluded.action,moved_to=excluded.moved_to,reason=excluded.reason",
            params![key, task, occurrence, action, moved_to, text(p, "reason"), now()],
        )?;
        self.record(actor, "recurrence.exceptions.create", Some(&task), p)?;
        self.rows(
            "SELECT * FROM recurrence_exceptions WHERE task_id=?1 AND occurrence_date=?2",
            &[&task, &occurrence],
        )
        .map(|v| v[0].clone())
    }

    pub fn recurrence_exceptions_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let task = require(p, "task_id")?;
        let occurrence = require(p, "occurrence_date")?;
        let n = self.conn.execute(
            "DELETE FROM recurrence_exceptions WHERE task_id=?1 AND occurrence_date=?2",
            [&task, &occurrence],
        )?;
        if n == 0 {
            return Err(LifeError::NotFound(format!("{task}:{occurrence}")));
        }
        self.record(actor, "recurrence.exceptions.delete", Some(&task), p)?;
        Ok(json!({"deleted":true,"task_id":task,"occurrence_date":occurrence}))
    }

    /// Expand a recurring task into the concrete occurrence dates inside a
    /// window. Exceptions skip or move single occurrences.
    pub fn recurrence_occurrences(&self, p: &Value) -> Result<Value> {
        let task = require(p, "task_id")?;
        let from = require(p, "from")?;
        let to = require(p, "to")?;
        let start = NaiveDate::parse_from_str(&from, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("from must be YYYY-MM-DD".into()))?;
        let end = NaiveDate::parse_from_str(&to, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("to must be YYYY-MM-DD".into()))?;
        if end < start {
            return Err(LifeError::Invalid("to must not precede from".into()));
        }
        let row = self
            .rows("SELECT * FROM tasks WHERE id=?1", &[&task])?
            .as_array()
            .and_then(|v| v.first())
            .cloned()
            .ok_or_else(|| LifeError::NotFound(task.clone()))?;
        let recurrence = row["recurrence"].as_str().unwrap_or("").to_string();
        if recurrence.is_empty() {
            let due = row["due_date"].as_str().unwrap_or("");
            let inside = due >= from.as_str() && due <= to.as_str();
            return Ok(json!({
                "task_id":task,
                "occurrences": if inside { json!([{"date":due,"state":"scheduled","moved":false}]) } else { json!([]) },
                "count": if inside { 1 } else { 0 }
            }));
        }
        let anchor = row["due_date"]
            .as_str()
            .and_then(|v| v.get(..10))
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .unwrap_or(start);
        let exceptions = self.rows(
            "SELECT occurrence_date,action,moved_to FROM recurrence_exceptions WHERE task_id=?1",
            &[&task],
        )?;
        let mut out: Vec<Value> = Vec::new();
        let mut cursor = anchor;
        let mut guard = 0;
        while cursor <= end && guard < 800 {
            guard += 1;
            if cursor >= start {
                let key = cursor.format("%Y-%m-%d").to_string();
                let exception = exceptions
                    .as_array()
                    .and_then(|rows| {
                        rows.iter()
                            .find(|r| r["occurrence_date"].as_str() == Some(key.as_str()))
                    })
                    .cloned();
                match exception {
                    Some(row) => {
                        let action = row["action"].as_str().unwrap_or("skip").to_string();
                        if action == "reschedule" {
                            if let Some(moved) = row["moved_to"].as_str() {
                                out.push(json!({"date":key,"state":"moved","moved":true,"moved_to":moved}));
                            }
                        } else {
                            out.push(json!({"date":key,"state":"skipped","moved":false}));
                        }
                    }
                    None => out.push(json!({"date":key,"state":"scheduled","moved":false})),
                }
            }
            cursor = match recurrence.as_str() {
                "daily" => cursor + Duration::days(1),
                "weekdays" => {
                    let mut d = cursor + Duration::days(1);
                    while matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
                        d += Duration::days(1);
                    }
                    d
                }
                "weekly" => cursor + Duration::days(7),
                "monthly" => {
                    let mut y = cursor.year();
                    let mut m = cursor.month();
                    m += 1;
                    if m > 12 {
                        m = 1;
                        y += 1;
                    }
                    let max = if m == 12 { 31 } else { 28 };
                    let first_next = NaiveDate::from_ymd_opt(
                        if m == 12 { y + 1 } else { y },
                        if m == 12 { 1 } else { m + 1 },
                        1,
                    )
                    .unwrap();
                    let last = (first_next - Duration::days(1)).day();
                    NaiveDate::from_ymd_opt(y, m, cursor.day().min(last.min(max))).unwrap()
                }
                _ => break,
            };
        }
        let count = out.len();
        Ok(json!({"task_id":task,"recurrence":recurrence,"occurrences":out,"count":count}))
    }

    pub fn calendar_sources_list(&self) -> Result<Value> {
        self.rows("SELECT * FROM calendar_sources ORDER BY name", &[])
    }

    pub fn calendar_sources_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let name = require(p, "name")?;
        let kind = optional(p, "kind").unwrap_or_else(|| "local".into());
        if !["local", "ics", "external"].contains(&kind.as_str()) {
            return Err(LifeError::Invalid(
                "kind must be local, ics or external".into(),
            ));
        }
        let key = id();
        let read_only = match p.get("read_only") {
            Some(Value::Bool(flag)) => i64::from(*flag),
            Some(Value::Number(number)) => number.as_i64().unwrap_or(0),
            _ => i64::from(kind != "local"),
        };
        let color = optional(p, "color").unwrap_or_else(|| "#8C9E90".into());
        self.conn.execute(
            "INSERT INTO calendar_sources(id,name,kind,read_only,color) VALUES(?1,?2,?3,?4,?5)",
            params![key, name, kind, read_only, color],
        )?;
        self.record(actor, "calendar.sources.create", Some(&key), p)?;
        self.rows("SELECT * FROM calendar_sources WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }

    pub fn calendar_sources_update(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let current = self
            .rows("SELECT * FROM calendar_sources WHERE id=?1", &[&key])?
            .as_array()
            .and_then(|v| v.first())
            .cloned()
            .ok_or_else(|| LifeError::NotFound(key.clone()))?;
        let name = p
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(current["name"].as_str().unwrap_or(""))
            .to_string();
        let read_only = match p.get("read_only") {
            Some(Value::Bool(flag)) => i64::from(*flag),
            Some(Value::Number(number)) => number.as_i64().unwrap_or(0),
            _ => current["read_only"].as_i64().unwrap_or(0),
        };
        let color = p
            .get("color")
            .and_then(Value::as_str)
            .unwrap_or(current["color"].as_str().unwrap_or("#8C9E90"))
            .to_string();
        self.conn.execute(
            "UPDATE calendar_sources SET name=?2,read_only=?3,color=?4 WHERE id=?1",
            params![key, name, read_only, color],
        )?;
        self.record(actor, "calendar.sources.update", Some(&key), p)?;
        self.rows("SELECT * FROM calendar_sources WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }

    pub fn calendar_sources_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let n = self
            .conn
            .execute("DELETE FROM calendar_sources WHERE id=?1", [&key])?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "calendar.sources.delete", Some(&key), p)?;
        Ok(json!({"deleted":true,"id":key}))
    }

    /// Events in a window, annotated with their source, all-day flag and
    /// stored IANA time zone.
    pub fn calendar_window(&self, p: &Value) -> Result<Value> {
        let from = require(p, "from")?;
        let to = require(p, "to")?;
        self.rows(
            "SELECT e.*,s.name AS source_name,s.kind AS source_kind,s.read_only AS source_read_only,s.color AS source_color FROM events e LEFT JOIN calendar_sources s ON s.name=e.import_source WHERE e.start_at<?2 AND e.end_at>?1 ORDER BY e.start_at",
            &[&from, &to],
        )
    }
}
