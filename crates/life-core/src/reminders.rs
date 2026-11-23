//! Persistent personal reminder schedule and deduplicated notification delivery.
//! Local-first, opt-in and independent of the graphical window.
use super::*;
use chrono::{NaiveDateTime, NaiveTime};
use rusqlite::OptionalExtension;
use std::collections::HashSet;

fn reminder_clock(date: &str, clock: &str) -> Option<NaiveDateTime> {
    let d = NaiveDate::parse_from_str(date.get(..10)?, "%Y-%m-%d").ok()?;
    let t = NaiveTime::parse_from_str(clock, "%H:%M").ok()?;
    Some(d.and_time(t))
}
fn reminder_timestamp(s: &str) -> Option<NaiveDateTime> {
    let raw = s.get(..19).unwrap_or(s);
    NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M"))
        .ok()
}
fn reminder_now(p: &Value) -> Result<NaiveDateTime> {
    if let Some(value) = optional(p, "now") {
        return reminder_timestamp(&value)
            .ok_or_else(|| LifeError::Invalid("now must be ISO local datetime".into()));
    }
    Ok(Local::now().naive_local())
}
fn reminder_date_time(s: NaiveDateTime) -> String {
    s.format("%Y-%m-%dT%H:%M:%S").to_string()
}
fn within_quiet(now: NaiveTime, start: &str, end: &str) -> Result<bool> {
    let a = NaiveTime::parse_from_str(start, "%H:%M")
        .map_err(|_| LifeError::Invalid("Invalid quiet hours start".into()))?;
    let b = NaiveTime::parse_from_str(end, "%H:%M")
        .map_err(|_| LifeError::Invalid("Invalid quiet hours end".into()))?;
    Ok(if a == b {
        false
    } else if a < b {
        now >= a && now < b
    } else {
        now >= a || now < b
    })
}
fn item(
    key: String,
    source: &str,
    source_id: &str,
    title: String,
    body: String,
    at: NaiveDateTime,
) -> Value {
    json!({"key":key,"source_kind":source,"source_id":source_id,"title":title,"body":body,"due_at":reminder_date_time(at)})
}
impl Store {
    pub fn reminders_settings(&self) -> Result<Value> {
        let raw = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key='notification.preferences'",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        Ok(match raw {
            Some(value) => serde_json::from_str(&value)
                .unwrap_or(json!({"enabled":false,"quiet_start":"22:00","quiet_end":"08:00"})),
            None => json!({"enabled":false,"quiet_start":"22:00","quiet_end":"08:00"}),
        })
    }
    pub fn reminders_upcoming(&self, p: &Value) -> Result<Value> {
        let now = reminder_now(p)?;
        let forward = integer(p, "forward_minutes", 10080);
        let lookback = integer(p, "lookback_minutes", 0);
        if !(0..=10080).contains(&forward) || !(0..=43200).contains(&lookback) {
            return Err(LifeError::Invalid(
                "Reminders window exceeds safe bounds".into(),
            ));
        }
        let from = now - Duration::minutes(lookback);
        let to = now + Duration::minutes(forward);
        let include_sent = p
            .get("include_sent")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut upcoming = Vec::<(NaiveDateTime, Value)>::new();
        let mut add = |candidate: Value| {
            let Some(trigger) = candidate["due_at"].as_str().and_then(reminder_timestamp) else {
                return;
            };
            upcoming.push((trigger, candidate));
        };
        let tasks=self.rows("SELECT id,title,due_date FROM tasks WHERE deleted_at IS NULL AND status NOT IN ('done','cancelled') AND due_date IS NOT NULL ORDER BY due_date ASC LIMIT 500",&[])?;
        for task in tasks.as_array().into_iter().flatten() {
            if let (Some(id), Some(day)) = (task["id"].as_str(), task["due_date"].as_str())
                && let Some(at) = reminder_clock(day, "09:00")
            {
                add(item(
                    format!("task:{id}:{day}"),
                    "task",
                    id,
                    task["title"].as_str().unwrap_or("Task").to_string(),
                    "Task deadline".into(),
                    at,
                ));
            }
        }
        let events=self.rows("SELECT id,title,start_at,all_day FROM events WHERE start_at IS NOT NULL ORDER BY start_at ASC LIMIT 500",&[])?;
        for event in events.as_array().into_iter().flatten() {
            if let (Some(id), Some(start)) = (event["id"].as_str(), event["start_at"].as_str()) {
                let at = if event["all_day"].as_i64() == Some(1) {
                    reminder_clock(start, "09:00")
                } else {
                    reminder_timestamp(start).map(|s| s - Duration::minutes(15))
                };
                if let Some(at) = at {
                    add(item(
                        format!("event:{id}:{start}"),
                        "event",
                        id,
                        event["title"]
                            .as_str()
                            .unwrap_or("Calendar commitment")
                            .to_string(),
                        if event["all_day"].as_i64() == Some(1) {
                            "All-day event".into()
                        } else {
                            "Starts in 15 minutes".into()
                        },
                        at,
                    ));
                }
            }
        }
        let careers=self.rows("SELECT id,employer,role,next_action_on,next_action FROM career_applications WHERE deleted_at IS NULL AND next_action_on IS NOT NULL AND stage NOT IN ('rejected','withdrawn','accepted') ORDER BY next_action_on ASC LIMIT 250",&[])?;
        for row in careers.as_array().into_iter().flatten() {
            if let (Some(id), Some(day)) = (row["id"].as_str(), row["next_action_on"].as_str())
                && let Some(at) = reminder_clock(day, "09:00")
            {
                add(item(
                    format!("career:{id}:{day}"),
                    "career",
                    id,
                    format!(
                        "{} · {}",
                        row["employer"].as_str().unwrap_or("Company"),
                        row["role"].as_str().unwrap_or("Role")
                    ),
                    row["next_action"]
                        .as_str()
                        .unwrap_or("Career follow-up")
                        .to_string(),
                    at,
                ));
            }
        }
        let financial=self.rows("SELECT id,title,due_date FROM finance_commitments WHERE deleted_at IS NULL AND status='pending' AND due_date IS NOT NULL ORDER BY due_date ASC LIMIT 250",&[])?;
        for row in financial.as_array().into_iter().flatten() {
            if let (Some(id), Some(day)) = (row["id"].as_str(), row["due_date"].as_str())
                && let Some(at) = reminder_clock(day, "09:00")
            {
                add(item(
                    format!("finance:{id}:{day}"),
                    "finance",
                    id,
                    row["title"].as_str().unwrap_or("Payment").to_string(),
                    "Financial commitment due".into(),
                    at,
                ));
            }
        }
        let learning=self.rows("SELECT id,title,due_date FROM learning_tracks WHERE deleted_at IS NULL AND status IN ('active','planned') AND due_date IS NOT NULL ORDER BY due_date ASC LIMIT 250",&[])?;
        for row in learning.as_array().into_iter().flatten() {
            if let (Some(id), Some(day)) = (row["id"].as_str(), row["due_date"].as_str())
                && let Some(at) = reminder_clock(day, "09:00")
            {
                add(item(
                    format!("learning:{id}:{day}"),
                    "learning",
                    id,
                    row["title"].as_str().unwrap_or("Learning").to_string(),
                    "Study milestone due".into(),
                    at,
                ));
            }
        }
        // Read ack state only for candidate keys; history never repeats a delivered item unless snoozed.
        let mut displayed = Vec::new();
        let mut seen = HashSet::new();
        for (at, mut candidate) in upcoming {
            let key = candidate["key"].as_str().unwrap_or("");
            if !seen.insert(key.to_string()) {
                continue;
            }
            let delivery=self.conn.query_row(
                "SELECT delivered_at,dismissed_at,snoozed_until FROM reminder_delivery WHERE key=?1",[key],
                |r|Ok((r.get::<_,Option<String>>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,Option<String>>(2)?))
            ).optional()?;
            if let Some((delivered, dismissed, snoozed)) = delivery {
                if !include_sent && (delivered.is_some() || dismissed.is_some()) {
                    continue;
                }
                if let Some(snoozed_until) = snoozed.and_then(|v| reminder_timestamp(&v)) {
                    candidate["due_at"] = json!(reminder_date_time(snoozed_until));
                }
            }
            let Some(effective) = candidate["due_at"].as_str().and_then(reminder_timestamp) else {
                continue;
            };
            if effective < from || effective > to {
                continue;
            }
            displayed.push((at, candidate));
        }
        displayed.sort_by(|a, b| {
            a.1["due_at"]
                .as_str()
                .cmp(&b.1["due_at"].as_str())
                .then_with(|| a.0.cmp(&b.0))
        });
        let limit = integer(p, "limit", 100).clamp(1, 500) as usize;
        Ok(json!(
            displayed
                .into_iter()
                .take(limit)
                .map(|(_, v)| v)
                .collect::<Vec<_>>()
        ))
    }
    pub fn reminders_due(&self, p: &Value) -> Result<Value> {
        let preferences = self.reminders_settings()?;
        if preferences["enabled"].as_bool() != Some(true) {
            return Ok(json!([]));
        }
        let now = reminder_now(p)?;
        let start = preferences["quiet_start"].as_str().unwrap_or("22:00");
        let end = preferences["quiet_end"].as_str().unwrap_or("08:00");
        if within_quiet(now.time(), start, end)? {
            return Ok(json!([]));
        }
        let mut options = p.clone();
        if !options.is_object() {
            options = json!({});
        }
        options["forward_minutes"] = json!(integer(p, "forward_minutes", 5).clamp(0, 60));
        options["lookback_minutes"] = json!(integer(p, "lookback_minutes", 1440).clamp(0, 43200));
        options["limit"] = json!(integer(p, "limit", 20).clamp(1, 50));
        let items = self.reminders_upcoming(&options)?;
        Ok(json!(
            items
                .as_array()
                .into_iter()
                .flatten()
                .filter(|item| {
                    item["due_at"]
                        .as_str()
                        .and_then(reminder_timestamp)
                        .is_some_and(|t| {
                            t <= now
                                + Duration::minutes(integer(p, "forward_minutes", 5).clamp(0, 60))
                        })
                })
                .cloned()
                .collect::<Vec<_>>()
        ))
    }
    pub fn reminders_claim(&self, p: &Value) -> Result<Value> {
        let key = require(p, "key")?;
        let now = reminder_now(p)?;
        let valid = self
            .reminders_due(p)?
            .as_array()
            .is_some_and(|v| v.iter().any(|r| r["key"] == key));
        if !valid {
            return Ok(
                json!({"key":key,"claimed":false,"reason":"not due, disabled, quiet hours, or already delivered"}),
            );
        }
        let lease = reminder_date_time(now + Duration::minutes(2));
        let now_str = reminder_date_time(now);
        let affected=self.conn.execute(
            "INSERT INTO reminder_delivery(key,claimed_until,updated_at) VALUES(?1,?2,?3) ON CONFLICT(key) DO UPDATE SET claimed_until=excluded.claimed_until,updated_at=excluded.updated_at WHERE delivered_at IS NULL AND dismissed_at IS NULL AND (claimed_until IS NULL OR claimed_until<=?3)",
            params![key,lease,now_str])?;
        Ok(json!({"key":key,"claimed":affected>0,"lease_until":lease}))
    }
    pub fn reminders_delivered(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "key")?;
        let now = reminder_date_time(reminder_now(p)?);
        let affected=self.conn.execute("UPDATE reminder_delivery SET delivered_at=?2,claimed_until=NULL,updated_at=?2 WHERE key=?1 AND claimed_until IS NOT NULL AND dismissed_at IS NULL",params![key,now])?;
        if affected == 0 {
            return Err(LifeError::Conflict(
                "Reminder was not leased or already delivered".into(),
            ));
        }
        self.record(actor, "reminders.delivered", None, &json!({"key":key}))?;
        Ok(json!({"key":key,"delivered":true}))
    }
    pub fn reminders_release(&self, p: &Value) -> Result<Value> {
        let key = require(p, "key")?;
        let affected=self.conn.execute("UPDATE reminder_delivery SET claimed_until=NULL,updated_at=?2 WHERE key=?1 AND delivered_at IS NULL",params![key,now()])?;
        Ok(json!({"key":key,"released":affected>0}))
    }
    pub fn reminders_dismiss(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "key")?;
        let timestamp = now();
        self.conn.execute("INSERT INTO reminder_delivery(key,dismissed_at,updated_at) VALUES(?1,?2,?2) ON CONFLICT(key) DO UPDATE SET dismissed_at=excluded.dismissed_at,claimed_until=NULL,updated_at=excluded.updated_at",params![key,timestamp])?;
        self.record(actor, "reminders.dismiss", None, &json!({"key":key}))?;
        Ok(json!({"key":key,"dismissed":true}))
    }
    pub fn reminders_snooze(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "key")?;
        let until = require(p, "until")?;
        let later = reminder_timestamp(&until)
            .ok_or_else(|| LifeError::Invalid("until must be ISO local datetime".into()))?;
        let current = reminder_now(p)?;
        if later <= current || later > current + Duration::days(90) {
            return Err(LifeError::Invalid(
                "Snooze must be in the future and within 90 days".into(),
            ));
        }
        self.conn.execute("INSERT INTO reminder_delivery(key,snoozed_until,updated_at) VALUES(?1,?2,?3) ON CONFLICT(key) DO UPDATE SET snoozed_until=excluded.snoozed_until,claimed_until=NULL,delivered_at=NULL,dismissed_at=NULL,updated_at=excluded.updated_at",params![key,reminder_date_time(later),now()])?;
        self.record(
            actor,
            "reminders.snooze",
            None,
            &json!({"key":key,"until":until}),
        )?;
        Ok(json!({"key":key,"snoozed_until":until}))
    }
    pub fn reminders_history(&self) -> Result<Value> {
        self.rows(
            "SELECT * FROM reminder_delivery ORDER BY updated_at DESC LIMIT 200",
            &[],
        )
    }
}
