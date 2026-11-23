//! FigCraft feature extensions: templates, saved views, measurable goals, links,
//! local availability, safer import and explainable reports.
use super::*;

fn checked_name(p: &Value, field: &str) -> Result<String> {
    let value = require(p, field)?;
    if value.chars().count() > 180 {
        return Err(LifeError::Invalid(format!(
            "{field} exceeds 180 characters"
        )));
    }
    Ok(value)
}
fn positive_id(p: &Value) -> Result<String> {
    require(p, "id")
}

impl Store {
    pub fn templates_list(&self) -> Result<Value> {
        self.rows(
            "SELECT * FROM task_templates WHERE archived=0 ORDER BY created_at DESC",
            &[],
        )
    }
    pub fn templates_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let name = checked_name(p, "name")?;
        let title = checked_name(p, "title")?;
        let priority = integer(p, "priority", 3);
        if !(1..=4).contains(&priority) {
            return Err(LifeError::Invalid("Priority must be 1–4".into()));
        }
        let estimate = integer(p, "estimate_minutes", 30);
        if !(1..=1440).contains(&estimate) {
            return Err(LifeError::Invalid("Estimate must be 1–1440 minutes".into()));
        }
        let key = id();
        let body = text(p, "description");
        self.conn.execute("INSERT INTO task_templates(id,name,title,description,priority,estimate_minutes,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![key,name,title,body,priority,estimate,now()])?;
        self.record(actor, "templates.create", Some(&key), p)?;
        self.rows("SELECT * FROM task_templates WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }
    pub fn templates_archive(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = positive_id(p)?;
        let n = self.conn.execute(
            "UPDATE task_templates SET archived=1 WHERE id=?1 AND archived=0",
            [&key],
        )?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "templates.delete", Some(&key), p)?;
        Ok(json!({"archived":true,"id":key}))
    }
    pub fn templates_apply(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let row = self.rows(
            "SELECT * FROM task_templates WHERE id=?1 AND archived=0",
            &[&key],
        )?;
        let template = row
            .as_array()
            .and_then(|a| a.first())
            .ok_or_else(|| LifeError::NotFound(key.clone()))?;
        let mut task = json!({"title":template["title"],"description":template["description"],"priority":template["priority"],"estimate_minutes":template["estimate_minutes"]});
        for k in ["project_id", "area_id", "due_date", "planned_date"] {
            if let Some(v) = p.get(k) {
                task[k] = v.clone()
            }
        }
        let output = self.create_task(&task, actor)?;
        self.record(actor, "templates.apply", output["id"].as_str(), p)?;
        Ok(output)
    }
    pub fn saved_views_list(&self, p: &Value) -> Result<Value> {
        let entity = optional(p, "entity").unwrap_or_else(|| "tasks".into());
        self.rows(
            "SELECT * FROM saved_filters WHERE entity=?1 ORDER BY created_at DESC",
            &[&entity],
        )
    }
    pub fn saved_views_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let name = checked_name(p, "name")?;
        let entity = optional(p, "entity").unwrap_or_else(|| "tasks".into());
        if !["tasks", "projects", "notes", "events"].contains(&entity.as_str()) {
            return Err(LifeError::Invalid("Unsupported filter entity".into()));
        }
        let query = p
            .get("query")
            .ok_or_else(|| LifeError::Invalid("query required".into()))?;
        if !query.is_object() {
            return Err(LifeError::Invalid("query must be an object".into()));
        }
        let allowed = [
            "status",
            "project_id",
            "area_id",
            "priority",
            "query",
            "due_from",
            "due_to",
        ];
        if query
            .as_object()
            .is_some_and(|q| q.keys().any(|k| !allowed.contains(&k.as_str())))
        {
            return Err(LifeError::Invalid("Unknown saved-filter property".into()));
        }
        let key = id();
        self.conn.execute("INSERT INTO saved_filters(id,entity,name,query_json,created_at) VALUES(?1,?2,?3,?4,?5)",params![key,entity,name,query.to_string(),now()])?;
        self.record(actor, "saved_views.create", Some(&key), p)?;
        self.rows("SELECT * FROM saved_filters WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }
    pub fn saved_views_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = positive_id(p)?;
        let n = self
            .conn
            .execute("DELETE FROM saved_filters WHERE id=?1", [&key])?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "saved_views.delete", Some(&key), p)?;
        Ok(json!({"id":key,"deleted":true}))
    }
    pub fn key_results_list(&self, p: &Value) -> Result<Value> {
        if let Some(goal_id) = optional(p, "goal_id") {
            self.rows(
                "SELECT * FROM key_results WHERE goal_id=?1 ORDER BY created_at",
                &[&goal_id],
            )
        } else {
            self.rows("SELECT * FROM key_results ORDER BY created_at", &[])
        }
    }
    pub fn key_results_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let goal_id = require(p, "goal_id")?;
        self.fetch("goals", &goal_id)?;
        let title = checked_name(p, "title")?;
        let baseline = p.get("baseline").and_then(Value::as_f64).unwrap_or(0.0);
        let target = p.get("target").and_then(Value::as_f64).unwrap_or(100.0);
        if !baseline.is_finite() || !target.is_finite() || target <= baseline {
            return Err(LifeError::Invalid(
                "target must be finite and greater than baseline".into(),
            ));
        }
        let unit = optional(p, "unit").unwrap_or_else(|| "units".into());
        let key = id();
        self.conn.execute("INSERT INTO key_results(id,goal_id,title,unit,baseline,target,current,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?5,?7,?7)",params![key,goal_id,title,unit,baseline,target,now()])?;
        self.record(actor, "key_results.create", Some(&key), p)?;
        self.rows("SELECT * FROM key_results WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }
    pub fn key_results_update(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let current = p
            .get("current")
            .and_then(Value::as_f64)
            .ok_or_else(|| LifeError::Invalid("current number required".into()))?;
        if !current.is_finite() {
            return Err(LifeError::Invalid("current must be finite".into()));
        }
        let previous = self.rows("SELECT * FROM key_results WHERE id=?1", &[&key])?;
        let old = previous
            .as_array()
            .and_then(|a| a.first())
            .ok_or_else(|| LifeError::NotFound(key.clone()))?;
        self.conn.execute(
            "UPDATE key_results SET current=?2,updated_at=?3 WHERE id=?1",
            params![key, current, now()],
        )?;
        self.conn.execute("INSERT INTO goal_progress_history(id,goal_id,key_result_id,previous,current,created_at) VALUES(?1,?2,?3,?4,?5,?6)",params![id(),old["goal_id"].as_str(),key,old["current"].as_f64(),current,now()])?;
        self.refresh_goal_okr(old["goal_id"].as_str().unwrap_or(""))?;
        self.record(actor, "key_results.update", Some(&key), p)?;
        self.rows("SELECT * FROM key_results WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }
    fn refresh_goal_okr(&self, goal_id: &str) -> Result<()> {
        let (count,avg):(i64,Option<f64>)=self.conn.query_row("SELECT COUNT(*),AVG(MAX(0.0,MIN(100.0,100.0*(current-baseline)/(target-baseline)))) FROM key_results WHERE goal_id=?1",[goal_id],|r|Ok((r.get(0)?,r.get(1)?)))?;
        if count > 0 {
            self.conn.execute(
                "UPDATE goals SET progress=?2 WHERE id=?1",
                params![goal_id, avg.unwrap_or(0.0)],
            )?;
        }
        Ok(())
    }
    pub fn key_results_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let row = self.rows("SELECT goal_id FROM key_results WHERE id=?1", &[&key])?;
        let goal_id = row
            .as_array()
            .and_then(|v| v.first())
            .and_then(|v| v["goal_id"].as_str())
            .ok_or_else(|| LifeError::NotFound(key.clone()))?
            .to_string();
        self.conn
            .execute("DELETE FROM key_results WHERE id=?1", [&key])?;
        self.refresh_goal_okr(&goal_id)?;
        self.record(actor, "key_results.delete", Some(&key), p)?;
        Ok(json!({"deleted":true,"id":key}))
    }
    pub fn goal_progress_history(&self, p: &Value) -> Result<Value> {
        let goal_id = require(p, "goal_id")?;
        self.rows("SELECT * FROM goal_progress_history WHERE goal_id=?1 ORDER BY created_at DESC LIMIT 200",&[&goal_id])
    }
}
impl Store {
    pub fn entity_links_list(&self, p: &Value) -> Result<Value> {
        let kind = require(p, "kind")?;
        let key = require(p, "id")?;
        self.rows("SELECT * FROM entity_links WHERE (source_kind=?1 AND source_id=?2) OR (target_kind=?1 AND target_id=?2) ORDER BY created_at DESC",&[&kind,&key])
    }
    pub fn entity_links_add(&self, p: &Value, actor: &str) -> Result<Value> {
        let source_kind = require(p, "source_kind")?;
        let source_id = require(p, "source_id")?;
        let target_kind = require(p, "target_kind")?;
        let target_id = require(p, "target_id")?;
        for (kind, key) in [(&source_kind, &source_id), (&target_kind, &target_id)] {
            if !["tasks", "notes", "projects", "goals"].contains(&kind.as_str()) {
                return Err(LifeError::Invalid("Unsupported linked entity type".into()));
            }
            self.fetch(kind, key)?;
        }
        if source_kind == target_kind && source_id == target_id {
            return Err(LifeError::Invalid("A record cannot link to itself".into()));
        }
        let relation = optional(p, "relation").unwrap_or_else(|| "related".into());
        if !["related", "reference", "supports", "note", "backlink"].contains(&relation.as_str()) {
            return Err(LifeError::Invalid("Unsupported relation".into()));
        }
        let key = id();
        self.conn.execute("INSERT INTO entity_links(id,source_kind,source_id,target_kind,target_id,relation,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![key,source_kind,source_id,target_kind,target_id,relation,now()])?;
        self.record(actor, "links.add", Some(&key), p)?;
        self.rows("SELECT * FROM entity_links WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }
    pub fn entity_links_remove(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let count = self
            .conn
            .execute("DELETE FROM entity_links WHERE id=?1", [&key])?;
        if count == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "links.remove", Some(&key), p)?;
        Ok(json!({"id":key,"deleted":true}))
    }
    pub fn availability_list(&self) -> Result<Value> {
        self.rows(
            "SELECT * FROM availability_rules ORDER BY weekday,start_time",
            &[],
        )
    }
    pub fn availability_replace(&self, p: &Value, actor: &str) -> Result<Value> {
        let rules = p
            .get("rules")
            .and_then(Value::as_array)
            .ok_or_else(|| LifeError::Invalid("rules array required".into()))?;
        if rules.len() > 100 {
            return Err(LifeError::Invalid(
                "Maximum 100 weekly availability slots".into(),
            ));
        }
        for v in rules {
            let weekday = v
                .get("weekday")
                .and_then(Value::as_i64)
                .ok_or_else(|| LifeError::Invalid("weekday must be 0–6 (Monday first)".into()))?;
            if !(0..=6).contains(&weekday) {
                return Err(LifeError::Invalid("weekday must be 0–6".into()));
            }
            let a = require(v, "start_time")?;
            let b = require(v, "end_time")?;
            let valid = |s: &str| {
                let bs = s.as_bytes();
                bs.len() == 5
                    && bs[2] == b':'
                    && bs[0].is_ascii_digit()
                    && bs[1].is_ascii_digit()
                    && bs[3].is_ascii_digit()
                    && bs[4].is_ascii_digit()
                    && &s[..2] < "24"
                    && &s[3..] < "60"
            };
            if !valid(&a) || !valid(&b) || b <= a {
                return Err(LifeError::Invalid(
                    "Availability needs valid HH:MM interval and end after start".into(),
                ));
            }
            let kind = optional(v, "kind").unwrap_or_else(|| "available".into());
            if !["available", "protected", "break", "unavailable"].contains(&kind.as_str()) {
                return Err(LifeError::Invalid("Invalid availability kind".into()));
            }
        }
        self.conn.execute("DELETE FROM availability_rules", [])?;
        for rule in rules {
            self.conn.execute("INSERT INTO availability_rules(id,weekday,start_time,end_time,kind) VALUES(?1,?2,?3,?4,?5)",params![id(),rule["weekday"].as_i64(),text(rule,"start_time"),text(rule,"end_time"),optional(rule,"kind").unwrap_or_else(||"available".into())])?;
        }
        self.record(actor, "availability.replace", None, p)?;
        self.availability_list()
    }
    pub fn tasks_bulk(&self, p: &Value, actor: &str) -> Result<Value> {
        let ids = p
            .get("ids")
            .and_then(Value::as_array)
            .ok_or_else(|| LifeError::Invalid("ids array required".into()))?;
        if ids.is_empty() || ids.len() > 100 {
            return Err(LifeError::Invalid(
                "Bulk updates require 1–100 task IDs".into(),
            ));
        }
        let fields = p
            .get("fields")
            .and_then(Value::as_object)
            .ok_or_else(|| LifeError::Invalid("fields object required".into()))?;
        let allowed = [
            "status",
            "priority",
            "due_date",
            "planned_date",
            "area_id",
            "project_id",
        ];
        if fields.is_empty() || fields.keys().any(|k| !allowed.contains(&k.as_str())) {
            return Err(LifeError::Invalid("Unsupported bulk update fields".into()));
        }
        let mut seen = std::collections::HashSet::new();
        let mut updated = Vec::new();
        for key in ids {
            let key = key
                .as_str()
                .ok_or_else(|| LifeError::Invalid("Each ID must be a string".into()))?;
            if !seen.insert(key.to_string()) {
                return Err(LifeError::Invalid(
                    "Duplicate task ID in bulk operation".into(),
                ));
            }
            let mut item = Value::Object(fields.clone());
            item["id"] = json!(key);
            updated.push(self.update_task(&item, actor)?);
        }
        self.record(actor, "tasks.bulk", None, p)?;
        Ok(json!({"count":updated.len(),"updated":updated}))
    }
    pub fn tasks_snooze(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let date = require(p, "date")?;
        NaiveDate::parse_from_str(&date, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("date must be YYYY-MM-DD".into()))?;
        self.update_task(
            &json!({"id":key,"due_date":date,"expected_revision":p.get("expected_revision")}),
            actor,
        )
    }
    pub fn reports_summary(&self, p: &Value) -> Result<Value> {
        let from = optional(p, "from")
            .unwrap_or_else(|| (Local::now().date_naive() - Duration::days(7)).to_string());
        let to = optional(p, "to").unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
        let first = NaiveDate::parse_from_str(&from, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("from must be YYYY-MM-DD".into()))?;
        let last = NaiveDate::parse_from_str(&to, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("to must be YYYY-MM-DD".into()))?;
        if last < first || (last - first).num_days() > 366 {
            return Err(LifeError::Invalid(
                "Report interval must be chronological and at most 366 days".into(),
            ));
        }
        let complete:i64=self.conn.query_row("SELECT COUNT(*) FROM tasks WHERE completed_at >= ?1 AND completed_at < ?2 AND deleted_at IS NULL",params![format!("{from}T00:00:00"),format!("{}T00:00:00",last+Duration::days(1))],|r|r.get(0))?;
        let focus:i64=self.conn.query_row("SELECT COALESCE(SUM(duration_seconds),0) FROM focus_sessions WHERE status='completed' AND started_at >= ?1 AND started_at < ?2",params![format!("{from}T00:00:00"),format!("{}T00:00:00",last+Duration::days(1))],|r|r.get(0))?;
        let habit_logs: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM habit_logs WHERE day >= ?1 AND day <= ?2 AND value > 0",
            params![from, to],
            |r| r.get(0),
        )?;
        let planned:i64=self.conn.query_row("SELECT COALESCE(SUM(estimate_minutes),0) FROM tasks WHERE deleted_at IS NULL AND planned_date>=?1 AND planned_date<=?2 AND status!='cancelled'",params![from,to],|r|r.get(0))?;
        Ok(
            json!({"from":from,"to":to,"completed_tasks":complete,"focused_seconds":focus,"logged_habit_days":habit_logs,"planned_task_minutes":planned,"source":"SQLite recorded task completion, focus intervals, habit logs and task estimates; no inferred productivity score"}),
        )
    }
}
impl Store {
    pub fn import_preview(&self, p: &Value) -> Result<Value> {
        let items = p
            .get("items")
            .and_then(Value::as_array)
            .ok_or_else(|| LifeError::Invalid("items array required".into()))?;
        if items.len() > 500 {
            return Err(LifeError::Invalid(
                "Maximum 500 rows per import; split larger files".into(),
            ));
        }
        let mut valid = Vec::new();
        let mut invalid = Vec::new();
        let mut duplicates = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let kind = text(item, "type");
            let label = if kind == "projects" {
                text(item, "name")
            } else {
                text(item, "title")
            };
            let reason = if !["tasks", "notes", "projects", "goals", "events", "habits"]
                .contains(&kind.as_str())
            {
                Some(
                    "Unsupported type; use tasks, notes, projects, goals, events or habits"
                        .to_string(),
                )
            } else if label.is_empty() || label.chars().count() > 300 {
                Some("Title/name is required and must not exceed 300 characters".to_string())
            } else if kind == "events"
                && (text(item, "start_at").is_empty()
                    || text(item, "end_at") <= text(item, "start_at"))
            {
                Some("Event start and end are required; end must be later".to_string())
            } else {
                None
            };
            if let Some(reason) = reason {
                invalid.push(json!({"index":index,"type":kind,"reason":reason}));
                continue;
            }
            let table = kind.as_str();
            let column = if kind == "projects" { "name" } else { "title" };
            let sql = format!("SELECT COUNT(*) FROM {table} WHERE {column}=?1");
            let existing: i64 = self.conn.query_row(&sql, [&label], |r| r.get(0))?;
            if existing > 0 {
                duplicates.push(json!({"index":index,"type":kind,"title":label,"reason":"Matching title/name already exists"}));
                continue;
            }
            valid.push(json!({"index":index,"type":kind,"title":label}));
        }
        Ok(
            json!({"total":items.len(),"valid":valid,"invalid":invalid,"duplicates":duplicates,"can_apply":true,"policy":"On apply, valid rows are committed atomically; duplicates are skipped; invalid rows require explicit allow_invalid=true."}),
        )
    }
    pub fn import_apply(&mut self, p: &Value, actor: &str) -> Result<Value> {
        if p.get("confirm").and_then(Value::as_bool) != Some(true) {
            return Err(LifeError::Invalid(
                "confirm=true required after preview".into(),
            ));
        }
        let preview = self.import_preview(p)?;
        let invalid = preview["invalid"].as_array().cloned().unwrap_or_default();
        if !invalid.is_empty() && p.get("allow_invalid").and_then(Value::as_bool) != Some(true) {
            return Err(LifeError::Invalid(format!(
                "{} invalid import rows; review preview and set allow_invalid=true to skip",
                invalid.len()
            )));
        }
        let items = p["items"]
            .as_array()
            .ok_or_else(|| LifeError::Invalid("items array required".into()))?;
        let mut created = Vec::new();
        for item in preview["valid"].as_array().into_iter().flatten() {
            let index = item["index"].as_u64().unwrap_or(u64::MAX) as usize;
            let record = items
                .get(index)
                .ok_or_else(|| LifeError::Invalid("Invalid import index".into()))?;
            let kind = text(record, "type");
            let action = format!("{kind}.create");
            let result = self.dispatch(&action, record, actor)?;
            created.push(json!({"index":index,"type":kind,"id":result["id"]}));
        }
        self.record(actor,"import.apply",None,&json!({"created":created.len(),"skipped_duplicates":preview["duplicates"].as_array().map_or(0,Vec::len),"invalid":invalid.len()}))?;
        Ok(
            json!({"created":created,"skipped_duplicates":preview["duplicates"],"skipped_invalid":invalid,"atomic":true}),
        )
    }
    pub fn area_dashboard(&self, p: &Value) -> Result<Value> {
        let key = require(p, "id")?;
        let area = self.fetch("areas", &key)?;
        let tasks=self.rows("SELECT id,title,status,priority,due_date,planned_date,project_id FROM tasks WHERE area_id=?1 AND deleted_at IS NULL ORDER BY priority,due_date LIMIT 100",&[&key])?;
        let projects = self.rows(
            "SELECT * FROM projects WHERE area_id=?1 ORDER BY updated_at DESC",
            &[&key],
        )?;
        let goals = self.rows(
            "SELECT * FROM goals WHERE area_id=?1 ORDER BY created_at DESC",
            &[&key],
        )?;
        Ok(json!({"area":area,"tasks":tasks,"projects":projects,"goals":goals}))
    }
    pub fn project_timeline(&self, p: &Value) -> Result<Value> {
        let key = require(p, "id")?;
        let project = self.fetch("projects", &key)?;
        let tasks=self.rows("SELECT id,title,status,priority,planned_date,due_date,estimate_minutes,parent_id FROM tasks WHERE project_id=?1 AND deleted_at IS NULL ORDER BY COALESCE(planned_date,due_date),priority",&[&key])?;
        let milestones = self.rows(
            "SELECT * FROM milestones WHERE project_id=?1 ORDER BY due_date",
            &[&key],
        )?;
        Ok(json!({"project":project,"tasks":tasks,"milestones":milestones}))
    }
}
