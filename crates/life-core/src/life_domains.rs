//! Local-first personal life domains: career applications, learning plans,
//! lightweight finance commitments, and optional wellbeing reflections.
use super::*;
use rusqlite::{params_from_iter, types::Value as SqlValue};

fn domain_spec(
    kind: &str,
) -> Result<(
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
)> {
    match kind {
        "career" => Ok((
            "career_applications",
            &[
                "employer",
                "role",
                "location",
                "stage",
                "applied_on",
                "next_action",
                "next_action_on",
                "interview_at",
                "job_url",
                "notes",
                "salary_min",
                "salary_max",
                "currency",
            ],
            &["employer", "role"],
        )),
        "learning" => Ok((
            "learning_tracks",
            &[
                "title",
                "provider",
                "status",
                "progress",
                "target_hours",
                "due_date",
                "notes",
                "url",
            ],
            &["title"],
        )),
        "finance" => Ok((
            "finance_commitments",
            &[
                "title",
                "amount_minor",
                "currency",
                "due_date",
                "status",
                "category",
                "notes",
            ],
            &["title"],
        )),
        _ => Err(LifeError::Invalid("Unsupported life domain".into())),
    }
}
fn sql_value(v: &Value) -> Result<SqlValue> {
    Ok(match v {
        Value::Null => SqlValue::Null,
        Value::String(s) => SqlValue::Text(s.clone()),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SqlValue::Integer(i)
            } else {
                SqlValue::Real(
                    n.as_f64()
                        .ok_or_else(|| LifeError::Invalid("Invalid number".into()))?,
                )
            }
        }
        Value::Bool(b) => SqlValue::Integer(i64::from(*b)),
        _ => {
            return Err(LifeError::Invalid(
                "Domain fields must be scalar values".into(),
            ));
        }
    })
}
fn check_day(value: &Value, field: &str) -> Result<()> {
    if let Some(s) = value.get(field).and_then(Value::as_str)
        && !s.is_empty()
    {
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid(format!("{field} must be YYYY-MM-DD")))?;
    }
    Ok(())
}
fn valid_domain(kind: &str, v: &Value) -> Result<()> {
    let (_table, fields, required) = domain_spec(kind)?;
    for key in required {
        let s = v.get(*key).and_then(Value::as_str).unwrap_or("").trim();
        if s.is_empty() || s.chars().count() > 250 {
            return Err(LifeError::Invalid(format!(
                "{key} is required (1–250 characters)"
            )));
        }
    }
    for key in fields {
        if let Some(s) = v.get(*key).and_then(Value::as_str)
            && s.chars().count() > 12000
        {
            return Err(LifeError::Invalid(format!("{key} is too long")));
        }
    }
    if kind == "career" {
        let stage = v.get("stage").and_then(Value::as_str).unwrap_or("saved");
        if ![
            "saved",
            "applied",
            "screening",
            "interview",
            "offer",
            "rejected",
            "withdrawn",
            "accepted",
        ]
        .contains(&stage)
        {
            return Err(LifeError::Invalid("Unsupported career stage".into()));
        }
        for k in ["applied_on", "next_action_on"] {
            check_day(v, k)?;
        }
        if let Some(interview) = v.get("interview_at").and_then(Value::as_str)
            && !interview.is_empty()
            && chrono::DateTime::parse_from_rfc3339(interview).is_err()
            && chrono::NaiveDateTime::parse_from_str(interview, "%Y-%m-%dT%H:%M:%S").is_err()
        {
            return Err(LifeError::Invalid(
                "interview_at must be ISO datetime".into(),
            ));
        }
        for key in ["salary_min", "salary_max"] {
            if let Some(amount) = v.get(key).filter(|v| !v.is_null())
                && amount.as_i64().is_none_or(|x| x < 0)
            {
                return Err(LifeError::Invalid(format!(
                    "{key} must be a nonnegative integer"
                )));
            }
        }
        if let (Some(a), Some(b)) = (
            v.get("salary_min").and_then(Value::as_i64),
            v.get("salary_max").and_then(Value::as_i64),
        ) && b < a
        {
            return Err(LifeError::Invalid(
                "Maximum salary must not be below minimum".into(),
            ));
        }
    }
    if kind == "learning" {
        let status = v.get("status").and_then(Value::as_str).unwrap_or("planned");
        if !["planned", "active", "paused", "completed", "archived"].contains(&status) {
            return Err(LifeError::Invalid("Unsupported learning status".into()));
        }
        if let Some(progress) = v.get("progress").filter(|v| !v.is_null())
            && progress
                .as_f64()
                .is_none_or(|n| !n.is_finite() || !(0.0..=100.0).contains(&n))
        {
            return Err(LifeError::Invalid(
                "Progress must be between 0 and 100".into(),
            ));
        }
        if let Some(hours) = v.get("target_hours").filter(|v| !v.is_null())
            && hours
                .as_f64()
                .is_none_or(|n| !n.is_finite() || !(0.0..=100_000.0).contains(&n))
        {
            return Err(LifeError::Invalid(
                "Target hours must be nonnegative".into(),
            ));
        }
        check_day(v, "due_date")?;
    }
    if kind == "finance" {
        let status = v.get("status").and_then(Value::as_str).unwrap_or("pending");
        if !["pending", "paid", "cancelled"].contains(&status) {
            return Err(LifeError::Invalid("Unsupported payment status".into()));
        }
        if let Some(amount) = v.get("amount_minor").filter(|v| !v.is_null())
            && amount
                .as_i64()
                .is_none_or(|n| !(0..=9_000_000_000_000).contains(&n))
        {
            return Err(LifeError::Invalid(
                "amount_minor must be nonnegative integer minor currency units".into(),
            ));
        }
        let currency = v.get("currency").and_then(Value::as_str).unwrap_or("SGD");
        if currency.len() != 3 || !currency.bytes().all(|b| b.is_ascii_uppercase()) {
            return Err(LifeError::Invalid(
                "Currency must be a three-letter uppercase code".into(),
            ));
        }
        check_day(v, "due_date")?;
    }
    if let Some(url) = v
        .get("url")
        .or_else(|| v.get("job_url"))
        .and_then(Value::as_str)
        && !url.is_empty()
        && !(url.starts_with("https://") || url.starts_with("http://"))
    {
        return Err(LifeError::Invalid(
            "Only http/https links are allowed".into(),
        ));
    }
    Ok(())
}
impl Store {
    pub fn domain_list(&self, kind: &str, p: &Value) -> Result<Value> {
        let (table, _, _) = domain_spec(kind)?;
        if let Some(status) = optional(p, "status") {
            let sql = format!(
                "SELECT * FROM {table} WHERE deleted_at IS NULL AND {}=?1 ORDER BY updated_at DESC LIMIT 500",
                if kind == "career" { "stage" } else { "status" }
            );
            self.rows(&sql, &[&status])
        } else {
            self.rows(&format!("SELECT * FROM {table} WHERE deleted_at IS NULL ORDER BY updated_at DESC LIMIT 500"),&[])
        }
    }
    pub fn domain_get(&self, kind: &str, p: &Value) -> Result<Value> {
        let (table, _, _) = domain_spec(kind)?;
        let key = require(p, "id")?;
        let rows = self.rows(&format!("SELECT * FROM {table} WHERE id=?1"), &[&key])?;
        rows.as_array()
            .and_then(|a| a.first())
            .cloned()
            .ok_or_else(|| LifeError::NotFound(key))
    }
    pub fn domain_create(&self, kind: &str, p: &Value, actor: &str) -> Result<Value> {
        let (table, allowed, _) = domain_spec(kind)?;
        for key in p.as_object().into_iter().flat_map(|o| o.keys()) {
            if !allowed.contains(&key.as_str()) && !["actor", "request_id"].contains(&key.as_str())
            {
                return Err(LifeError::Invalid(format!("Unknown {kind} field: {key}")));
            }
        }
        valid_domain(kind, p)?;
        let key = id();
        let time = now();
        let mut cols = vec!["id".to_owned(), "created_at".into(), "updated_at".into()];
        let mut vals = vec![
            SqlValue::Text(key.clone()),
            SqlValue::Text(time.clone()),
            SqlValue::Text(time),
        ];
        for field in allowed {
            if let Some(value) = p.get(*field) {
                cols.push((*field).into());
                vals.push(sql_value(value)?);
            }
        }
        let args = (1..=vals.len()).map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("INSERT INTO {table}({}) VALUES({args})", cols.join(","));
        self.conn.execute(&query, params_from_iter(vals))?;
        self.record(actor, &format!("{kind}.create"), Some(&key), p)?;
        self.domain_get(kind, &json!({"id":key}))
    }
    pub fn domain_update(&self, kind: &str, p: &Value, actor: &str) -> Result<Value> {
        let (table, allowed, _) = domain_spec(kind)?;
        let key = require(p, "id")?;
        let existing = self.domain_get(kind, &json!({"id":key}))?;
        if !existing["deleted_at"].is_null() {
            return Err(LifeError::Conflict(
                "Record archived; restore before editing".into(),
            ));
        }
        if let Some(revision) = p.get("expected_revision").and_then(Value::as_i64)
            && existing["revision"].as_i64() != Some(revision)
        {
            return Err(LifeError::Conflict("Stale record revision".into()));
        }
        let mut combined = existing.clone();
        let mut columns = Vec::new();
        let mut values = Vec::new();
        for (field, value) in p.as_object().into_iter().flat_map(|o| o.iter()) {
            if ["id", "actor", "request_id", "expected_revision"].contains(&field.as_str()) {
                continue;
            }
            if !allowed.contains(&field.as_str()) {
                return Err(LifeError::Invalid(format!("Unknown {kind} field: {field}")));
            }
            combined[field] = value.clone();
            columns.push(format!("{field}=?"));
            values.push(sql_value(value)?);
        }
        if columns.is_empty() {
            return Ok(existing);
        }
        valid_domain(kind, &combined)?;
        columns.push("updated_at=?".into());
        values.push(SqlValue::Text(now()));
        values.push(SqlValue::Text(key.clone()));
        let sql = format!(
            "UPDATE {table} SET {},revision=revision+1 WHERE id=? AND deleted_at IS NULL",
            columns.join(",")
        );
        self.conn.execute(&sql, params_from_iter(values))?;
        self.record(actor, &format!("{kind}.update"), Some(&key), p)?;
        self.domain_get(kind, &json!({"id":key}))
    }
    pub fn domain_delete(&self, kind: &str, p: &Value, actor: &str) -> Result<Value> {
        let (table, _, _) = domain_spec(kind)?;
        let key = require(p, "id")?;
        let affected=self.conn.execute(&format!("UPDATE {table} SET deleted_at=?2,updated_at=?2,revision=revision+1 WHERE id=?1 AND deleted_at IS NULL"),params![key,now()])?;
        if affected == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, &format!("{kind}.delete"), Some(&key), p)?;
        Ok(json!({"id":key,"archived":true}))
    }
    pub fn domain_restore(&self, kind: &str, p: &Value, actor: &str) -> Result<Value> {
        let (table, _, _) = domain_spec(kind)?;
        let key = require(p, "id")?;
        let affected=self.conn.execute(&format!("UPDATE {table} SET deleted_at=NULL,updated_at=?2,revision=revision+1 WHERE id=?1 AND deleted_at IS NOT NULL"),params![key,now()])?;
        if affected == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, &format!("{kind}.restore"), Some(&key), p)?;
        self.domain_get(kind, &json!({"id":key}))
    }
    pub fn learning_log(&self, p: &Value, actor: &str) -> Result<Value> {
        let track_id = require(p, "track_id")?;
        let track = self.domain_get("learning", &json!({"id":track_id}))?;
        if !track["deleted_at"].is_null() {
            return Err(LifeError::Conflict(
                "Cannot log to an archived track".into(),
            ));
        }
        let minutes = integer(p, "minutes", 0);
        if !(1..=1440).contains(&minutes) {
            return Err(LifeError::Invalid(
                "Session minutes must be between 1 and 1440".into(),
            ));
        }
        let day = optional(p, "day").unwrap_or_else(|| Local::now().date_naive().to_string());
        NaiveDate::parse_from_str(&day, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("day must be YYYY-MM-DD".into()))?;
        let key = id();
        self.conn.execute("INSERT INTO learning_sessions(id,track_id,day,minutes,note,created_at) VALUES(?1,?2,?3,?4,?5,?6)",params![key,track_id,day,minutes,text(p,"note"),now()])?;
        self.record(actor, "learning.log", Some(&key), p)?;
        self.rows("SELECT * FROM learning_sessions WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }
    pub fn learning_sessions(&self, p: &Value) -> Result<Value> {
        let key = require(p, "track_id")?;
        self.domain_get("learning", &json!({"id":key}))?;
        self.rows("SELECT * FROM learning_sessions WHERE track_id=?1 ORDER BY day DESC,created_at DESC LIMIT 500",&[&key])
    }
    pub fn wellbeing_log(&self, p: &Value, actor: &str) -> Result<Value> {
        let day = optional(p, "day").unwrap_or_else(|| Local::now().date_naive().to_string());
        NaiveDate::parse_from_str(&day, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("day must be YYYY-MM-DD".into()))?;
        let energy = integer(p, "energy", 3);
        let mood = integer(p, "mood", 3);
        if !(1..=5).contains(&energy) || !(1..=5).contains(&mood) {
            return Err(LifeError::Invalid("Energy and mood must be 1–5".into()));
        }
        let hours = p.get("sleep_hours").and_then(Value::as_f64);
        if hours.is_some_and(|h| !h.is_finite() || !(0.0..=24.0).contains(&h)) {
            return Err(LifeError::Invalid("Sleep hours must be 0–24".into()));
        }
        if text(p, "note").len() > 5000 {
            return Err(LifeError::Invalid("Reflection too long".into()));
        }
        self.conn.execute("INSERT INTO wellbeing_checkins(id,day,energy,mood,sleep_hours,note,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?7) ON CONFLICT(day) DO UPDATE SET energy=excluded.energy,mood=excluded.mood,sleep_hours=excluded.sleep_hours,note=excluded.note,updated_at=excluded.updated_at",params![id(),day,energy,mood,hours,text(p,"note"),now()])?;
        self.record(actor, "wellbeing.log", None, p)?;
        self.rows("SELECT * FROM wellbeing_checkins WHERE day=?1", &[&day])
            .map(|v| v[0].clone())
    }
    pub fn wellbeing_list(&self, p: &Value) -> Result<Value> {
        let limit = integer(p, "limit", 60).clamp(1, 365);
        self.rows(
            "SELECT * FROM wellbeing_checkins ORDER BY day DESC LIMIT ?1",
            &[&limit],
        )
    }
    pub fn wellbeing_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        if self
            .conn
            .execute("DELETE FROM wellbeing_checkins WHERE id=?1", [&key])?
            == 0
        {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "wellbeing.delete", Some(&key), &json!({"id":key}))?;
        Ok(json!({"id":key,"deleted":true}))
    }
    pub fn life_domain_dashboard(&self) -> Result<Value> {
        let career=self.rows("SELECT stage,COUNT(*) AS count FROM career_applications WHERE deleted_at IS NULL GROUP BY stage",&[])?;
        let learning=self.rows("SELECT status,COUNT(*) AS count FROM learning_tracks WHERE deleted_at IS NULL GROUP BY status",&[])?;
        let upcoming=self.rows("SELECT id,employer,role,stage,next_action,next_action_on,interview_at FROM career_applications WHERE deleted_at IS NULL AND (next_action_on IS NOT NULL OR interview_at IS NOT NULL) ORDER BY COALESCE(next_action_on,substr(interview_at,1,10)) LIMIT 15",&[])?;
        let finance=self.rows("SELECT id,title,currency,amount_minor,due_date FROM finance_commitments WHERE deleted_at IS NULL AND status='pending' ORDER BY due_date LIMIT 15",&[])?;
        Ok(
            json!({"career":career,"learning":learning,"upcoming_career":upcoming,"pending_finance":finance}),
        )
    }
}
