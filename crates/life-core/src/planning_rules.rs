//! Planning rules: protected time, weekly time budgets, travel buffers,
use super::*;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};

fn clock_time(s: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M")
        .map_err(|_| LifeError::Invalid(format!("Invalid time {s}; expected HH:MM")))
}

impl Store {
    pub fn protected_blocks_list(&self, p: &Value) -> Result<Value> {
        let weekday = optional(p, "weekday");
        match weekday {
            Some(day) => self.rows(
                "SELECT * FROM protected_blocks WHERE weekday=?1 ORDER BY start_time",
                &[&day],
            ),
            None => self.rows(
                "SELECT * FROM protected_blocks ORDER BY weekday,start_time",
                &[],
            ),
        }
    }

    pub fn protected_blocks_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let title = require(p, "title")?;
        let weekday = integer(p, "weekday", -1);
        if !(0..=6).contains(&weekday) {
            return Err(LifeError::Invalid(
                "weekday must be 0 (Monday) through 6 (Sunday)".into(),
            ));
        }
        let start = require(p, "start_time")?;
        let end = require(p, "end_time")?;
        let a = clock_time(&start)?;
        let b = clock_time(&end)?;
        if b <= a {
            return Err(LifeError::Invalid(
                "Protected time must end after it starts".into(),
            ));
        }
        let kind = optional(p, "kind").unwrap_or_else(|| "protected".into());
        let key = id();
        self.conn.execute(
            "INSERT INTO protected_blocks(id,title,weekday,start_time,end_time,kind,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![key, title, weekday, start, end, kind, now()],
        )?;
        self.record(actor, "protected.create", Some(&key), p)?;
        self.rows("SELECT * FROM protected_blocks WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }

    pub fn protected_blocks_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let n = self
            .conn
            .execute("DELETE FROM protected_blocks WHERE id=?1", [&key])?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "protected.delete", Some(&key), p)?;
        Ok(json!({"deleted":true,"id":key}))
    }

    pub fn time_budgets_list(&self) -> Result<Value> {
        self.rows(
            "SELECT * FROM time_budgets ORDER BY scope_kind,scope_id",
            &[],
        )
    }

    pub fn time_budgets_set(&self, p: &Value, actor: &str) -> Result<Value> {
        let scope_kind = optional(p, "scope_kind").unwrap_or_else(|| "global".into());
        if !["area", "project", "global"].contains(&scope_kind.as_str()) {
            return Err(LifeError::Invalid(
                "scope_kind must be area, project or global".into(),
            ));
        }
        let scope_id = optional(p, "scope_id");
        if scope_kind != "global" && scope_id.is_none() {
            return Err(LifeError::Invalid(
                "scope_id is required for area and project budgets".into(),
            ));
        }
        let period = optional(p, "period").unwrap_or_else(|| "week".into());
        if !["day", "week", "month"].contains(&period.as_str()) {
            return Err(LifeError::Invalid(
                "period must be day, week or month".into(),
            ));
        }
        let minutes = integer(p, "minutes", -1);
        if !(0..=100_000).contains(&minutes) {
            return Err(LifeError::Invalid(
                "minutes must be 0 through 100000".into(),
            ));
        }
        let key = id();
        self.conn.execute(
            "INSERT INTO time_budgets(id,scope_kind,scope_id,period,minutes,created_at) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(scope_kind,scope_id,period) DO UPDATE SET minutes=excluded.minutes",
            params![key, scope_kind, scope_id, period, minutes, now()],
        )?;
        self.record(actor, "time_budgets.set", Some(&key), p)?;
        self.rows("SELECT * FROM time_budgets WHERE scope_kind=?1 AND period=?2 AND IFNULL(scope_id,'')=IFNULL(?3,'')", &[&scope_kind, &period, &scope_id])
            .map(|v| v[0].clone())
    }

    pub fn time_budgets_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let n = self
            .conn
            .execute("DELETE FROM time_budgets WHERE id=?1", [&key])?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "time_budgets.delete", Some(&key), p)?;
        Ok(json!({"deleted":true,"id":key}))
    }

    /// Planned minutes per life area for the current week, compared with the
    /// configured weekly budget, and the resulting overrun.
    pub fn time_budget_report(&self, p: &Value) -> Result<Value> {
        let week = optional(p, "week_start").unwrap_or_else(|| {
            let today = Local::now().date_naive();
            let back = i64::from(today.weekday().num_days_from_monday());
            (today - Duration::days(back))
                .format("%Y-%m-%d")
                .to_string()
        });
        let start = NaiveDate::parse_from_str(&week, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("week_start must be YYYY-MM-DD".into()))?;
        let end = start + Duration::days(7);
        let budgets = self.rows(
            "SELECT * FROM time_budgets WHERE period='week' AND scope_kind='area'",
            &[],
        )?;
        let planned = self.rows(
            "SELECT area_id, SUM(estimate_minutes) AS minutes FROM tasks WHERE deleted_at IS NULL AND status NOT IN ('done','cancelled') AND planned_date>=?1 AND planned_date<?2 GROUP BY area_id",
            &[&start.format("%Y-%m-%d").to_string(), &end.format("%Y-%m-%d").to_string()],
        )?;
        let mut rows: Vec<Value> = Vec::new();
        let mut total_planned = 0i64;
        let mut total_budget = 0i64;
        for area in self
            .rows(
                "SELECT id,name FROM areas WHERE archived=0 ORDER BY name",
                &[],
            )?
            .as_array()
            .into_iter()
            .flatten()
        {
            let area_id = area["id"].as_str().unwrap_or("");
            let minutes = planned
                .as_array()
                .into_iter()
                .flatten()
                .find(|r| r["area_id"].as_str() == Some(area_id))
                .and_then(|r| r["minutes"].as_i64())
                .unwrap_or(0);
            let budget = budgets
                .as_array()
                .into_iter()
                .flatten()
                .find(|r| r["scope_id"].as_str() == Some(area_id))
                .and_then(|r| r["minutes"].as_i64())
                .unwrap_or(0);
            total_planned += minutes;
            total_budget += budget;
            rows.push(json!({
                "area_id":area_id,
                "area_name":area["name"].as_str().unwrap_or(""),
                "planned_minutes":minutes,
                "budget_minutes":budget,
                "overrun_minutes":(minutes - budget).max(0),
                "status": if budget == 0 { "unbudgeted" } else if minutes > budget { "overrun" } else { "within_budget" }
            }));
        }
        let unplanned = self.rows(
            "SELECT id,title,estimate_minutes,area_id FROM tasks WHERE deleted_at IS NULL AND status NOT IN ('done','cancelled') AND planned_date>=?1 AND planned_date<?2 AND (area_id IS NULL OR area_id='') AND planned_date IS NOT NULL",
            &[&start.format("%Y-%m-%d").to_string(), &end.format("%Y-%m-%d").to_string()],
        )?;
        Ok(json!({
            "week_start":start.format("%Y-%m-%d").to_string(),
            "week_end":end.format("%Y-%m-%d").to_string(),
            "areas":rows,
            "unplanned_tasks":unplanned,
            "total_planned_minutes":total_planned,
            "total_budget_minutes":total_budget,
            "overrun_minutes":(total_planned - total_budget).max(0),
            "source":"Local SQLite: task planned dates, estimates and configured weekly area budgets."
        }))
    }

    /// Unfinished work from a period that was not completed, for explicit
    /// carryover decisions.
    pub fn planning_carryover(&self, p: &Value) -> Result<Value> {
        let from = require(p, "from")?;
        let to = require(p, "to")?;
        let rows = self.rows(
            "SELECT id,title,status,priority,due_date,planned_date,estimate_minutes,project_id,area_id FROM tasks WHERE deleted_at IS NULL AND status NOT IN ('done','cancelled') AND (IFNULL(planned_date,due_date) IS NOT NULL AND IFNULL(planned_date,due_date)<?1) ORDER BY priority ASC",
            &[&to],
        )?;
        let count = rows.as_array().map_or(0, Vec::len);
        Ok(
            json!({"from":from,"to":to,"tasks":rows,"count":count,"source":"Local SQLite: open tasks whose planned or due date falls before the given period."}),
        )
    }

    /// Move a set of unfinished tasks into the next period.
    pub fn planning_carryover_apply(&self, p: &Value, actor: &str) -> Result<Value> {
        let ids = p
            .get("ids")
            .and_then(Value::as_array)
            .ok_or_else(|| LifeError::Invalid("ids array is required".into()))?;
        let target = require(p, "target_date")?;
        NaiveDate::parse_from_str(&target, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("target_date must be YYYY-MM-DD".into()))?;
        let mut moved = Vec::new();
        for value in ids {
            let key = value.as_str().unwrap_or("");
            if key.is_empty() {
                continue;
            }
            self.rows(
                "SELECT id FROM tasks WHERE id=?1 AND deleted_at IS NULL",
                &[&key],
            )?
            .as_array()
            .and_then(|v| v.first())
            .cloned()
            .ok_or_else(|| LifeError::NotFound(key.to_string()))?;
            self.conn.execute(
                "UPDATE tasks SET planned_date=?2,updated_at=?3,revision=revision+1 WHERE id=?1",
                params![key, target, now()],
            )?;
            self.record(actor, "planning.carryover.apply", Some(key), p)?;
            moved.push(key.to_string());
        }
        Ok(json!({"moved":moved,"count":moved.len(),"target_date":target}))
    }

    /// Travel and buffer minutes for a proposed schedule, honouring the
    /// configured transition buffer.
    pub fn planning_buffers(&self, p: &Value) -> Result<Value> {
        let buffer = integer(p, "buffer_minutes", 10);
        if !(0..=60).contains(&buffer) {
            return Err(LifeError::Invalid(
                "buffer_minutes must be 0 through 60".into(),
            ));
        }
        let travel = integer(p, "travel_minutes", 0);
        if !(0..=180).contains(&travel) {
            return Err(LifeError::Invalid(
                "travel_minutes must be 0 through 180".into(),
            ));
        }
        let date = optional(p, "date").unwrap_or_else(|| Local::now().date_naive().to_string());
        let proposal = self.plan_capacity(&json!({"date":date,"buffer_minutes":buffer}))?;
        let slots = proposal["slots"].clone();
        Ok(json!({
            "date":date,
            "buffer_minutes":buffer,
            "travel_minutes":travel,
            "slots":slots,
            "note":"Buffers and travel are applied between consecutive blocks; protected time is never overwritten without review.",
        }))
    }

    /// A protected-time aware weekly preparation summary for the next week.
    pub fn planning_next_week(&self, p: &Value) -> Result<Value> {
        let from = optional(p, "from").unwrap_or_else(|| {
            let today = Local::now().date_naive();
            let back = i64::from(today.weekday().num_days_from_monday());
            (today - Duration::days(back) + Duration::days(7))
                .format("%Y-%m-%d")
                .to_string()
        });
        let start = NaiveDate::parse_from_str(&from, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("from must be YYYY-MM-DD".into()))?;
        let end = start + Duration::days(7);
        let from_text = start.format("%Y-%m-%d").to_string();
        let end_text = end.format("%Y-%m-%d").to_string();
        let due = self.rows(
            "SELECT id,title,priority,due_date,estimate_minutes FROM tasks WHERE deleted_at IS NULL AND status NOT IN ('done','cancelled') AND due_date>=?1 AND due_date<?2 ORDER BY priority ASC",
            &[&from_text, &end_text],
        )?;
        let protected_blocks = self.rows(
            "SELECT * FROM protected_blocks WHERE weekday>=0 ORDER BY weekday,start_time",
            &[],
        )?;
        let capacity = self.plan_capacity(&json!({"date":from_text}))?;
        Ok(json!({
            "week_start":from_text,
            "week_end":end_text,
            "due_next_week":due,
            "protected_blocks":protected_blocks,
            "first_day_capacity_minutes":capacity["capacity_minutes"].clone(),
            "source":"Local SQLite: upcoming due tasks, protected time rules and computed availability.",
        }))
    }
}

fn _unused(_: NaiveDateTime) {}
