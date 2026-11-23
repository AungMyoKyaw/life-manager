//! Guided morning, evening, weekly and monthly reviews with evidence snapshots.
//! Persist drafts and generate canonical review records on completion.
use super::*;
use chrono::Datelike;

const MORNING: &[(&str, &str)] = &[
    ("intention", "What matters most today?"),
    ("priority", "Which one action would move things forward?"),
    ("protected_time", "What time needs protection?"),
    (
        "obstacles",
        "What could distract you and how will you respond?",
    ),
];
const EVENING: &[(&str, &str)] = &[
    ("wins", "What went well today?"),
    ("friction", "What slowed you down?"),
    ("recovery", "How can you rest and recover?"),
    ("tomorrow", "What is one clear next step tomorrow?"),
];
const WEEKLY: &[(&str, &str)] = &[
    ("wins", "What moved forward this week?"),
    ("unfinished", "What still deserves attention?"),
    ("capacity", "Did commitments fit your real capacity?"),
    ("adjustments", "What will you change next week?"),
];
const MONTHLY: &[(&str, &str)] = &[
    ("progress", "How did your longer-term goals progress?"),
    ("lessons", "What did you learn this month?"),
    ("balance", "Where did your energy go?"),
    (
        "direction",
        "Which direction deserves attention next month?",
    ),
];
fn prompts(kind: &str) -> Result<&'static [(&'static str, &'static str)]> {
    match kind {
        "morning" => Ok(MORNING),
        "evening" => Ok(EVENING),
        "weekly" => Ok(WEEKLY),
        "monthly" => Ok(MONTHLY),
        _ => Err(LifeError::Invalid(
            "Review kind must be morning, evening, weekly or monthly".into(),
        )),
    }
}
fn day_from(p: &Value) -> Result<NaiveDate> {
    let day = optional(p, "day").unwrap_or_else(|| Local::now().date_naive().to_string());
    NaiveDate::parse_from_str(&day, "%Y-%m-%d")
        .map_err(|_| LifeError::Invalid("Review date must be YYYY-MM-DD".into()))
}
fn review_window(kind: &str, day: NaiveDate) -> (NaiveDate, NaiveDate) {
    match kind {
        "weekly" => {
            let start = day - Duration::days(day.weekday().num_days_from_monday().into());
            (start, start + Duration::days(6))
        }
        "monthly" => {
            let start = day.with_day(1).unwrap_or(day);
            (start, day)
        }
        _ => (day, day),
    }
}
impl Store {
    pub fn review_guides_prompts(&self) -> Result<Value> {
        let kinds = ["morning", "evening", "weekly", "monthly"];
        Ok(Value::Array(kinds.iter().map(|k|json!({
            "kind":k,
            "prompts":prompts(k).unwrap_or(&[]).iter().map(|(id,prompt)|json!({"id":id,"prompt":prompt})).collect::<Vec<_>>()
        })).collect()))
    }
    pub fn review_guides_list(&self, p: &Value) -> Result<Value> {
        let kind = optional(p, "kind").unwrap_or_default();
        if !kind.is_empty() {
            prompts(&kind)?;
        }
        self.rows(
            "SELECT id,kind,day,state,answers_json,metrics_json,canonical_review_id,created_at,updated_at
             FROM guided_reviews WHERE ?1='' OR kind=?1 ORDER BY day DESC,updated_at DESC LIMIT 120",
            &[&kind],
        )
    }
    pub fn review_guides_get(&self, p: &Value) -> Result<Value> {
        let key = require(p, "id")?;
        let rows = self.rows("SELECT * FROM guided_reviews WHERE id=?1", &[&key])?;
        rows.as_array()
            .and_then(|r| r.first())
            .cloned()
            .ok_or(LifeError::NotFound(key))
    }
    pub fn review_guides_start(&self, p: &Value, actor: &str) -> Result<Value> {
        let kind = require(p, "kind")?;
        prompts(&kind)?;
        let day = day_from(p)?;
        let key = id();
        let (start, end) = review_window(&kind, day);
        let metrics =
            self.reports_summary(&json!({"from":start.to_string(),"to":end.to_string()}))?;
        let timestamp = now();
        self.conn.execute(
            "INSERT OR IGNORE INTO guided_reviews(id,kind,day,state,answers_json,metrics_json,created_at,updated_at)
             VALUES(?1,?2,?3,'draft','{}',?4,?5,?5)",
            params![key,kind,day.to_string(),metrics.to_string(),timestamp],
        )?;
        let result = self.rows(
            "SELECT * FROM guided_reviews WHERE kind=?1 AND day=?2",
            &[&kind, &day.to_string()],
        )?;
        let row = result
            .as_array()
            .and_then(|v| v.first())
            .cloned()
            .ok_or_else(|| LifeError::NotFound(format!("review:{kind}:{day}")))?;
        if row["id"] == key {
            self.record(actor, "review_guides.start", Some(&key), p)?;
        }
        Ok(row)
    }
    pub fn review_guides_update(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let row = self.review_guides_get(&json!({"id":key}))?;
        if row["state"] != "draft" {
            return Err(LifeError::Conflict(
                "Cannot edit completed review; start a new date".into(),
            ));
        }
        let kind = require(&row, "kind")?;
        let updates = p["answers"]
            .as_object()
            .ok_or_else(|| LifeError::Invalid("answers object required".into()))?;
        let mut answers: serde_json::Map<String, Value> =
            serde_json::from_str(row["answers_json"].as_str().unwrap_or("{}"))?;
        let valid_ids: std::collections::HashSet<&str> =
            prompts(&kind)?.iter().map(|(key, _)| *key).collect();
        for (id, answer) in updates {
            if !valid_ids.contains(id.as_str()) {
                return Err(LifeError::Invalid(format!("Unknown question ID: {id}")));
            }
            let text = answer
                .as_str()
                .ok_or_else(|| LifeError::Invalid(format!("Answer {id} must be text")))?;
            if text.chars().count() > 4000 {
                return Err(LifeError::Invalid(format!(
                    "Answer {id} exceeds 4000 characters"
                )));
            }
            answers.insert(id.clone(), Value::String(text.trim().to_owned()));
        }
        self.conn.execute(
            "UPDATE guided_reviews SET answers_json=?2,updated_at=?3 WHERE id=?1",
            params![key, Value::Object(answers).to_string(), now()],
        )?;
        self.record(actor, "review_guides.update", Some(&key), p)?;
        self.review_guides_get(&json!({"id":key}))
    }
    pub fn review_guides_complete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let row = self.review_guides_get(&json!({"id":key}))?;
        if row["state"] != "draft" {
            return Err(LifeError::Conflict(
                "Review has already been completed".into(),
            ));
        }
        let kind = require(&row, "kind")?;
        let day = require(&row, "day")?;
        let answers: Value = serde_json::from_str(row["answers_json"].as_str().unwrap_or("{}"))?;
        let pairs = prompts(&kind)?;
        if !pairs
            .iter()
            .any(|(k, _)| answers[*k].as_str().is_some_and(|s| !s.trim().is_empty()))
        {
            return Err(LifeError::Invalid(
                "Answer at least one reflection question first".into(),
            ));
        }
        let title = match kind.as_str() {
            "morning" => "Morning planning",
            "evening" => "Evening reflection",
            "weekly" => "Weekly reflection",
            _ => "Monthly review",
        };
        let mut content = String::new();
        for (k, question) in pairs {
            let answer = answers[*k].as_str().unwrap_or("");
            if !answer.is_empty() {
                content.push_str("## ");
                content.push_str(question);
                content.push_str("\n\n");
                content.push_str(answer);
                content.push_str("\n\n");
            }
        }
        let metrics: Value = serde_json::from_str(row["metrics_json"].as_str().unwrap_or("{}"))?;
        content.push_str("## Recorded context\n\n");
        content.push_str(&format!(
            "- Completed tasks: {}\n- Focused seconds: {}\n- Logged habit days: {}\n\n",
            metrics["completed_tasks"], metrics["focused_seconds"], metrics["logged_habit_days"]
        ));
        content.push_str("_Metrics are snapshots from local SQLite, not a productivity score._\n");
        let period = if kind == "weekly" {
            "weekly"
        } else if kind == "monthly" {
            "monthly"
        } else {
            "daily"
        };
        let canonical = self.create_review(
            &json!({
                "title":format!("{title} · {day}"),"period":period,"day":day,"content":content
            }),
            actor,
        )?;
        let canonical_id = require(&canonical, "id")?;
        self.conn.execute(
            "UPDATE guided_reviews SET state='complete',canonical_review_id=?2,updated_at=?3
             WHERE id=?1",
            params![key, canonical_id, now()],
        )?;
        self.record(actor, "review_guides.complete", Some(&key), p)?;
        Ok(json!({"guided":self.review_guides_get(&json!({"id":key}))?,
                  "canonical":canonical}))
    }
}
