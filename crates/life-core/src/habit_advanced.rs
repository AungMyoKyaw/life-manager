//! Weekly quotas, voluntary rest and explicit skip handling without coercive streak scores.
use super::*;
use chrono::Datelike;
impl Store {
    pub fn habit_configure(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        self.fetch("habits", &key)?;
        let quota = integer(p, "weekly_quota", 1);
        if !(1..=7).contains(&quota) {
            return Err(LifeError::Invalid("weekly_quota must be 1–7".into()));
        }
        let period = optional(p, "quota_period").unwrap_or_else(|| "weekly".into());
        if !["weekly", "daily"].contains(&period.as_str()) {
            return Err(LifeError::Invalid(
                "quota_period must be daily or weekly".into(),
            ));
        }
        let days = p.get("rest_days").cloned().unwrap_or_else(|| json!([]));
        let Some(days) = days.as_array() else {
            return Err(LifeError::Invalid(
                "rest_days must be a weekday array".into(),
            ));
        };
        let mut seen = std::collections::BTreeSet::new();
        for day in days {
            let Some(d) = day.as_u64() else {
                return Err(LifeError::Invalid(
                    "rest_days must contain weekday integers 0–6".into(),
                ));
            };
            if d > 6 || !seen.insert(d) {
                return Err(LifeError::Invalid(
                    "rest_days must be unique weekday integers 0–6".into(),
                ));
            }
        }
        if period == "weekly" && quota as usize > 7 - days.len() {
            return Err(LifeError::Invalid(
                "Weekly quota exceeds available days after configured rest".into(),
            ));
        }
        self.conn.execute(
            "UPDATE habits SET quota_period=?2,weekly_quota=?3,rest_days=?4 WHERE id=?1",
            params![key, period, quota, json!(days).to_string()],
        )?;
        self.record(actor, "habits.configure", Some(&key), p)?;
        self.fetch("habits", &key)
    }
    pub fn habit_skip(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "habit_id")?;
        self.fetch("habits", &key)?;
        let day = optional(p, "day").unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
        NaiveDate::parse_from_str(&day, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("day must be YYYY-MM-DD".into()))?;
        let status = optional(p, "status").unwrap_or_else(|| "skipped".into());
        if !["skipped", "rest"].contains(&status.as_str()) {
            return Err(LifeError::Invalid("status must be skipped or rest".into()));
        }
        let note = optional(p, "note").unwrap_or_default();
        if note.len() > 500 {
            return Err(LifeError::Invalid("skip note exceeds 500 bytes".into()));
        }
        self.conn.execute("INSERT INTO habit_logs(id,habit_id,day,value,created_at,status,note) VALUES(?1,?2,?3,0,?4,?5,?6) ON CONFLICT(habit_id,day) DO UPDATE SET value=0,status=excluded.status,note=excluded.note",params![id(),key,day,now(),status,note])?;
        self.record(actor, "habits.skip", Some(&key), p)?;
        Ok(json!({"habit_id":key,"day":day,"status":status,"value":0,"note":note}))
    }
    pub fn habit_weekly_progress(&self, p: &Value) -> Result<Value> {
        let day = optional(p, "day").unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
        let date = NaiveDate::parse_from_str(&day, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("day must be YYYY-MM-DD".into()))?;
        let monday = date - Duration::days(date.weekday().num_days_from_monday() as i64);
        let sunday = monday + Duration::days(6);
        let start = monday.to_string();
        let end = sunday.to_string();
        let rows=self.rows("SELECT h.id,h.title,h.frequency,h.quota_period,h.weekly_quota,h.rest_days,h.target,h.unit,COALESCE(SUM(CASE WHEN l.value>0 AND l.status='done' THEN 1 ELSE 0 END),0) AS completed_days,COALESCE(SUM(CASE WHEN l.status='skipped' THEN 1 ELSE 0 END),0) AS skipped_days,COALESCE(SUM(CASE WHEN l.status='rest' THEN 1 ELSE 0 END),0) AS rest_days_taken FROM habits h LEFT JOIN habit_logs l ON l.habit_id=h.id AND l.day>=?1 AND l.day<=?2 WHERE h.archived=0 GROUP BY h.id ORDER BY h.title",&[&start,&end])?;
        let enriched = rows
            .as_array()
            .into_iter()
            .flatten()
            .cloned()
            .map(|mut row| {
                let completed = row["completed_days"].as_i64().unwrap_or(0);
                let quota = if row["quota_period"] == "weekly" {
                    row["weekly_quota"].as_i64().unwrap_or(1)
                } else {
                    7
                };
                row["quota_for_period"] = json!(quota);
                row["fulfilled"] = json!(completed >= quota);
                row["week_start"] = json!(start);
                row["week_end"] = json!(end);
                row
            })
            .collect::<Vec<_>>();
        Ok(json!(enriched))
    }
}
