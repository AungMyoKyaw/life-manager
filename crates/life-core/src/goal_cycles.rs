//! Annual direction, quarterly planning and a deliberate three-priority limit.
use super::*;
use chrono::Datelike;

impl Store {
    fn goal_cycle_period(p: &Value) -> Result<(String, String, String, String)> {
        let kind = require(p, "kind")?;
        if !["year", "quarter"].contains(&kind.as_str()) {
            return Err(LifeError::Invalid(
                "Goal cycle kind must be year or quarter".into(),
            ));
        }
        let year = p
            .get("year")
            .and_then(Value::as_i64)
            .unwrap_or(Local::now().year() as i64);
        if !(1900..=2200).contains(&year) {
            return Err(LifeError::Invalid("Year must be 1900–2200".into()));
        }
        let (key, from, to) = if kind == "year" {
            (
                year.to_string(),
                format!("{year}-01-01"),
                format!("{year}-12-31"),
            )
        } else {
            let quarter = p
                .get("quarter")
                .and_then(Value::as_i64)
                .unwrap_or(((Local::now().month0() / 3) + 1) as i64);
            if !(1..=4).contains(&quarter) {
                return Err(LifeError::Invalid("Quarter must be 1–4".into()));
            }
            let start_month = (quarter - 1) * 3 + 1;
            let end_month = start_month + 2;
            let end_day = if end_month == 3 || end_month == 12 {
                31
            } else if end_month == 6 || end_month == 9 {
                30
            } else {
                31
            };
            (
                format!("{year}-Q{quarter}"),
                format!("{year}-{start_month:02}-01"),
                format!("{year}-{end_month:02}-{end_day:02}"),
            )
        };
        Ok((kind, key, from, to))
    }
    pub fn goal_cycles_list(&self, p: &Value) -> Result<Value> {
        let kind = optional(p, "kind").unwrap_or_default();
        if !kind.is_empty() && !["year", "quarter"].contains(&kind.as_str()) {
            return Err(LifeError::Invalid("Invalid cycle kind".into()));
        }
        self.rows(
            "SELECT * FROM goal_cycles WHERE (?1='' OR kind=?1) AND (?2=1 OR status!='archived')
             ORDER BY starts_at DESC,kind DESC",
            &[
                &kind,
                &p.get("include_archived")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ],
        )
    }
    pub fn goal_cycles_get(&self, p: &Value) -> Result<Value> {
        let key = require(p, "id")?;
        let result = self.rows("SELECT * FROM goal_cycles WHERE id=?1", &[&key])?;
        let cycle = result
            .as_array()
            .and_then(|x| x.first())
            .ok_or_else(|| LifeError::NotFound(key.clone()))?;
        let prioritized = self.rows(
            "SELECT g.id,g.title,g.status,g.progress,g.target,g.due_date,i.rank
             FROM goal_cycle_items i JOIN goals g ON g.id=i.goal_id
             WHERE i.cycle_id=?1 ORDER BY i.rank",
            &[&key],
        )?;
        Ok(json!({"cycle":cycle,"goals":prioritized,"max_priorities":3,
                  "source":"Explicitly ranked goals; progress values come from goals/key results"}))
    }
    pub fn goal_cycles_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let (kind, period_key, start, end) = Self::goal_cycle_period(p)?;
        let title = optional(p, "title").unwrap_or_else(|| {
            if kind == "year" {
                format!("{period_key} direction")
            } else {
                format!("{period_key} priorities")
            }
        });
        if title.trim().is_empty() || title.chars().count() > 180 {
            return Err(LifeError::Invalid(
                "Cycle title must be 1–180 characters".into(),
            ));
        }
        let existing: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM goal_cycles WHERE period_key=?1 AND kind=?2",
            params![period_key, kind],
            |r| r.get(0),
        )?;
        if existing > 0 {
            return Err(LifeError::Conflict(
                "This planning period already exists".into(),
            ));
        }
        let key = id();
        let ts = now();
        self.conn.execute(
            "INSERT INTO goal_cycles(id,kind,period_key,title,description,starts_at,ends_at,status,created_at,updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,'active',?8,?8)",
            params![key,kind,period_key,title,text(p,"description"),start,end,ts],
        )?;
        self.record(actor, "goal_cycles.create", Some(&key), p)?;
        self.goal_cycles_get(&json!({"id":key}))
    }
    pub fn goal_cycles_update(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let existing = self.goal_cycles_get(&json!({"id":key}))?;
        let current = &existing["cycle"];
        let title = optional(p, "title")
            .unwrap_or_else(|| current["title"].as_str().unwrap_or("").to_string());
        let description = optional(p, "description")
            .unwrap_or_else(|| current["description"].as_str().unwrap_or("").to_string());
        let status = optional(p, "status")
            .unwrap_or_else(|| current["status"].as_str().unwrap_or("active").to_string());
        if title.trim().is_empty() || title.chars().count() > 180 {
            return Err(LifeError::Invalid("Invalid cycle title".into()));
        }
        if !["active", "complete", "archived"].contains(&status.as_str()) {
            return Err(LifeError::Invalid("Unknown goal cycle status".into()));
        }
        self.conn.execute(
            "UPDATE goal_cycles SET title=?2,description=?3,status=?4,updated_at=?5 WHERE id=?1",
            params![key, title, description, status, now()],
        )?;
        self.record(actor, "goal_cycles.update", Some(&key), p)?;
        self.goal_cycles_get(&json!({"id":key}))
    }
    pub fn goal_cycles_prioritize(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let cycle = self.goal_cycles_get(&json!({"id":key}))?;
        if cycle["cycle"]["status"] != "active" {
            return Err(LifeError::Conflict(
                "Only active planning periods can be changed".into(),
            ));
        }
        let selected = p["goal_ids"]
            .as_array()
            .ok_or_else(|| LifeError::Invalid("goal_ids array required".into()))?;
        if selected.len() > 3 {
            return Err(LifeError::Invalid(
                "At most three focus goals per planning period".into(),
            ));
        }
        let mut seen = std::collections::HashSet::<String>::new();
        for v in selected {
            let gid = v
                .as_str()
                .ok_or_else(|| LifeError::Invalid("Each goal ID must be a string".into()))?;
            if !seen.insert(gid.to_owned()) {
                return Err(LifeError::Invalid("Duplicate goal ID".into()));
            }
            let goal = self.fetch("goals", gid)?;
            if goal["status"] != "active" {
                return Err(LifeError::Conflict(
                    "Cannot prioritize an inactive goal".into(),
                ));
            }
        }
        self.conn
            .execute("DELETE FROM goal_cycle_items WHERE cycle_id=?1", [&key])?;
        for (rank, gid) in selected.iter().enumerate() {
            self.conn.execute(
                "INSERT INTO goal_cycle_items(cycle_id,goal_id,rank) VALUES(?1,?2,?3)",
                params![key, gid.as_str(), rank + 1],
            )?;
        }
        self.record(actor, "goal_cycles.prioritize", Some(&key), p)?;
        self.goal_cycles_get(&json!({"id":key}))
    }
}
