//! The remaining workflow boards: task activity timelines, overdue decisions,
//! project blockers and notes, goal review and OKR detail, journal archives,
//! daily planning sessions, capture parsing and the edge-state services.
use super::*;
use chrono::Datelike;

impl Store {
    // --- task activity timeline --------------------------------------------

    /// The ordered history of one entity, from the activity ledger and the
    /// agent change ledger.
    pub fn entity_timeline(&self, p: &Value) -> Result<Value> {
        let kind = require(p, "kind")?;
        let key = require(p, "id")?;
        let limit = integer(p, "limit", 100).clamp(1, 500);
        let history = self.rows(
            "SELECT id,actor,action,entity_id,payload,created_at FROM action_history WHERE entity_id=?1 ORDER BY created_at DESC LIMIT ?2",
            &[&key, &limit],
        )?;
        let changes = self.rows(
            "SELECT id,actor,action,entity_id,request_id,before_json,after_json,undoable,undone_at,created_at FROM action_changes WHERE entity_id=?1 ORDER BY created_at DESC LIMIT ?2",
            &[&key, &limit],
        )?;
        let entries: Vec<Value> = history
            .as_array()
            .into_iter()
            .flatten()
            .map(|row| {
                json!({
                    "kind":"activity",
                    "actor":row["actor"].clone(),
                    "action":row["action"].clone(),
                    "at":row["created_at"].clone(),
                    "detail":row["payload"].clone()
                })
            })
            .chain(changes.as_array().into_iter().flatten().map(|row| {
                json!({
                    "kind":"change",
                    "actor":row["actor"].clone(),
                    "action":row["action"].clone(),
                    "at":row["created_at"].clone(),
                    "change_id":row["id"].clone(),
                    "undoable":row["undoable"].clone(),
                    "undone":!row["undone_at"].is_null(),
                    "before":row["before_json"].clone(),
                    "after":row["after_json"].clone()
                })
            }))
            .collect();
        let count = entries.len();
        Ok(json!({
            "kind":kind,
            "id":key,
            "entries":entries,
            "count":count,
            "source":"Local SQLite: the activity ledger and the agent change ledger."
        }))
    }

    /// The overdue decision: every overdue task with the choices that apply.
    pub fn overdue_decisions(&self, p: &Value) -> Result<Value> {
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        let scope = optional(p, "scope").unwrap_or_else(|| "all".into());
        let rows = self.rows(
            "SELECT t.*,p.name AS project_name,a.name AS area_name FROM tasks t LEFT JOIN projects p ON t.project_id=p.id LEFT JOIN areas a ON t.area_id=a.id WHERE t.deleted_at IS NULL AND t.status NOT IN ('done','cancelled') AND t.due_date IS NOT NULL AND t.due_date<?1 ORDER BY t.due_date ASC LIMIT ?2",
            &[&today, &integer(p, "limit", 100).clamp(1, 500)],
        )?;
        let mut decisions: Vec<Value> = Vec::new();
        for row in rows.as_array().into_iter().flatten() {
            let days = NaiveDate::parse_from_str(&today, "%Y-%m-%d")
                .ok()
                .zip(row["due_date"].as_str().and_then(|v| {
                    NaiveDate::parse_from_str(v.get(..10).unwrap_or(v), "%Y-%m-%d").ok()
                }))
                .map(|(today, due)| (today - due).num_days())
                .unwrap_or(0);
            decisions.push(json!({
                "task_id":row["id"].clone(),
                "title":row["title"].clone(),
                "priority":row["priority"].clone(),
                "status":row["status"].clone(),
                "due_date":row["due_date"].clone(),
                "project":row["project_name"].clone(),
                "area":row["area_name"].clone(),
                "days_overdue":days,
                "options":["snooze","reschedule","reprioritise","block","keep"]
            }));
        }
        let count = decisions.len();
        Ok(json!({
            "today":today,
            "scope":scope,
            "decisions":decisions,
            "count":count,
            "source":"Local SQLite: open tasks whose due date is before today."
        }))
    }

    /// Apply an overdue decision to one task.
    pub fn overdue_decide(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let decision = optional(p, "decision").unwrap_or_else(|| "snooze".into());
        let task = self
            .rows(
                "SELECT * FROM tasks WHERE id=?1 AND deleted_at IS NULL",
                &[&key],
            )?
            .as_array()
            .and_then(|v| v.first())
            .cloned()
            .ok_or_else(|| LifeError::NotFound(key.clone()))?;
        match decision.as_str() {
            "snooze" => {
                let date = require(p, "date")?;
                NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                    .map_err(|_| LifeError::Invalid("date must be YYYY-MM-DD".into()))?;
                self.conn.execute(
                    "UPDATE tasks SET due_date=?2,updated_at=?3,revision=revision+1 WHERE id=?1",
                    params![key, date, now()],
                )?;
                self.record(actor, "tasks.snooze", Some(&key), p)?;
                Ok(json!({"task_id":key,"decision":"snooze","due_date":date}))
            }
            "reschedule" => {
                let date = require(p, "date")?;
                NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                    .map_err(|_| LifeError::Invalid("date must be YYYY-MM-DD".into()))?;
                self.conn.execute(
                    "UPDATE tasks SET planned_date=?2,due_date=NULL,updated_at=?3,revision=revision+1 WHERE id=?1",
                    params![key, date, now()],
                )?;
                self.record(actor, "tasks.reschedule", Some(&key), p)?;
                Ok(json!({"task_id":key,"decision":"reschedule","planned_date":date}))
            }
            "reprioritise" => {
                let priority = integer(p, "priority", -1);
                if !(1..=4).contains(&priority) {
                    return Err(LifeError::Invalid("priority must be 1 through 4".into()));
                }
                self.conn.execute(
                    "UPDATE tasks SET priority=?2,updated_at=?3,revision=revision+1 WHERE id=?1",
                    params![key, priority, now()],
                )?;
                self.record(actor, "tasks.reprioritise", Some(&key), p)?;
                Ok(json!({"task_id":key,"decision":"reprioritise","priority":priority}))
            }
            "block" => {
                self.conn.execute(
                    "UPDATE tasks SET status='blocked',updated_at=?2,revision=revision+1 WHERE id=?1",
                    params![key, now()],
                )?;
                self.record(actor, "tasks.block", Some(&key), p)?;
                Ok(json!({"task_id":key,"decision":"block","status":"blocked"}))
            }
            "keep" => {
                self.record(actor, "tasks.keep_overdue", Some(&key), p)?;
                Ok(json!({"task_id":key,"decision":"keep","due_date":task["due_date"].clone()}))
            }
            _ => Err(LifeError::Invalid(
                "decision must be snooze, reschedule, reprioritise, block or keep".into(),
            )),
        }
    }

    // --- project blockers and notes ----------------------------------------

    /// Every blocked task with the predecessors that block it.
    pub fn project_blockers(&self, p: &Value) -> Result<Value> {
        let project = optional(p, "project_id");
        let rows = self.rows(
            "SELECT t.id,t.title,t.status,t.priority,t.due_date,t.estimate_minutes,d.task_id AS blocker_task_id,d.blocks_id AS blocked_task_id,bt.title AS blocker_title,bt.status AS blocker_status,bt.due_date AS blocker_due FROM task_dependencies d JOIN tasks t ON t.id=d.task_id JOIN tasks bt ON bt.id=d.blocks_id WHERE t.deleted_at IS NULL AND bt.deleted_at IS NULL AND EXISTS (WITH RECURSIVE reach(id) AS (SELECT blocks_id FROM task_dependencies WHERE task_id=?1 UNION SELECT d2.blocks_id FROM task_dependencies d2 JOIN reach r ON d2.task_id=r.id) SELECT 1 FROM reach WHERE id=bt.id) ORDER BY t.priority ASC",
            &[&project.clone().unwrap_or_default()],
        )?;
        let blockers: Vec<Value> = rows
            .as_array()
            .into_iter()
            .flatten()
            .map(|row| {
                json!({
                    "task_id":row["id"].clone(),
                    "title":row["title"].clone(),
                    "status":row["status"].clone(),
                    "priority":row["priority"].clone(),
                    "due_date":row["due_date"].clone(),
                    "blocked_by":row["blocker_title"].clone(),
                    "blocker_task_id":row["blocker_task_id"].clone(),
                    "blocker_status":row["blocker_status"].clone(),
                    "blocker_due_date":row["blocker_due_date"].clone()
                })
            })
            .collect();
        let count = blockers.len();
        Ok(json!({
            "project_id":project,
            "blockers":blockers,
            "count":count,
            "source":"Local SQLite: the task dependency graph and the tasks it points at."
        }))
    }

    /// Notes attached to a project.
    pub fn project_notes(&self, p: &Value) -> Result<Value> {
        let project = require(p, "project_id")?;
        let notes = self.rows(
            "SELECT id,title,content,updated_at FROM notes WHERE area_id IS NULL AND id IN (SELECT source_id FROM entity_links WHERE source_kind='notes' AND target_kind='projects' AND target_id=?1) ORDER BY updated_at DESC",
            &[&project],
        )?;
        let count = notes.as_array().map_or(0, Vec::len);
        Ok(json!({"project_id":project,"notes":notes,"count":count}))
    }

    /// Record a project outcome and archive the project.
    pub fn project_complete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let project = self.fetch("projects", &key)?;
        if project["status"].as_str() == Some("completed") {
            return Ok(
                json!({"project_id":key,"status":"completed","outcome":project["description"].clone()}),
            );
        }
        let outcome = optional(p, "outcome").unwrap_or_default();
        let description = if outcome.is_empty() {
            project["description"].as_str().unwrap_or("").to_string()
        } else {
            outcome
        };
        self.conn.execute(
            "UPDATE projects SET status='completed',description=?2,updated_at=?3 WHERE id=?1",
            params![key, description, now()],
        )?;
        self.record(actor, "projects.complete", Some(&key), p)?;
        Ok(json!({"project_id":key,"status":"completed","outcome":description}))
    }

    /// Reopen a paused or completed project.
    pub fn project_reopen(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        self.fetch("projects", &key)?;
        self.conn.execute(
            "UPDATE projects SET status='active',updated_at=?2 WHERE id=?1",
            params![key, now()],
        )?;
        self.record(actor, "projects.reopen", Some(&key), p)?;
        self.fetch("projects", &key)
    }

    // --- goals ---------------------------------------------------------------

    /// The monthly goal review: progress, evidence and the next action.
    pub fn goal_monthly_review(&self, p: &Value) -> Result<Value> {
        let month = optional(p, "month")
            .unwrap_or_else(|| Local::now().date_naive().format("%Y-%m").to_string());
        let goals = self.rows(
            "SELECT g.id,g.title,g.status,g.progress,g.target,g.due_date,a.name AS area FROM goals g LEFT JOIN areas a ON g.area_id=a.id WHERE g.status IN ('active','paused') ORDER BY g.progress DESC",
            &[],
        )?;
        let mut items: Vec<Value> = Vec::new();
        for goal in goals.as_array().into_iter().flatten() {
            let goal_id = goal["id"].as_str().unwrap_or("");
            let results = self.rows(
                "SELECT title,unit,baseline,target,current FROM key_results WHERE goal_id=?1",
                &[&goal_id],
            )?;
            let history = self.rows(
                "SELECT previous,current,created_at FROM goal_progress_history WHERE goal_id=?1 ORDER BY created_at DESC LIMIT 8",
                &[&goal_id],
            )?;
            items.push(json!({
                "goal_id":goal_id,
                "title":goal["title"].clone(),
                "area":goal["area"].clone(),
                "status":goal["status"].clone(),
                "progress":goal["progress"].clone(),
                "due_date":goal["due_date"].clone(),
                "key_results":results,
                "history":history
            }));
        }
        Ok(json!({
            "month":month,
            "goals":items,
            "count":items.len(),
            "source":"Local SQLite: active goals, their measurable key results and their progress history."
        }))
    }

    /// One goal with its key results, history and linked work.
    pub fn goal_detail(&self, p: &Value) -> Result<Value> {
        let key = require(p, "id")?;
        let goal = self.fetch("goals", &key)?;
        let results = self.rows(
            "SELECT * FROM key_results WHERE goal_id=?1 ORDER BY created_at",
            &[&key],
        )?;
        let history = self.rows(
            "SELECT * FROM goal_progress_history WHERE goal_id=?1 ORDER BY created_at DESC",
            &[&key],
        )?;
        let tasks = self.rows(
            "SELECT id,title,status,priority,due_date FROM tasks WHERE goal_id=?1 AND deleted_at IS NULL ORDER BY status,priority",
            &[&key],
        )?;
        Ok(json!({
            "goal":goal,
            "key_results":results,
            "history":history,
            "tasks":tasks
        }))
    }

    // --- reviews --------------------------------------------------------------

    /// The journal archive, grouped by period.
    pub fn journal_archive(&self, p: &Value) -> Result<Value> {
        let from = optional(p, "from");
        let to = optional(p, "to");
        let entries = self.rows(
            "SELECT id,period,day,title,content,created_at FROM reviews WHERE (?1='' OR day>=?1) AND (?2='' OR day<=?2) ORDER BY day DESC,created_at DESC LIMIT ?3",
            &[&from.clone().unwrap_or_default(), &to.clone().unwrap_or_default(), &integer(p, "limit", 200).clamp(1, 1000)],
        )?;
        let mut periods: Vec<Value> = Vec::new();
        for entry in entries.as_array().into_iter().flatten() {
            let period = entry["period"].as_str().unwrap_or("daily").to_string();
            if let Some(existing) = periods
                .iter_mut()
                .find(|p| p["period"].as_str() == Some(period.as_str()))
            {
                existing["count"] = json!(existing["count"].as_i64().unwrap_or(0) + 1);
            } else {
                periods.push(json!({"period":period,"count":1}));
            }
        }
        let count = entries.as_array().map_or(0, Vec::len);
        Ok(json!({
            "from":from,
            "to":to,
            "entries":entries,
            "periods":periods,
            "count":count
        }))
    }

    // --- planning -------------------------------------------------------------

    /// A daily planning session: the top three priorities, the day's
    /// commitments and the available capacity, with the chosen priorities
    /// stored against the day.
    pub fn planning_session(&self, p: &Value) -> Result<Value> {
        let date = optional(p, "date")
            .unwrap_or_else(|| Local::now().date_naive().format("%Y-%m-%d").to_string());
        let requested: Vec<String> = p
            .get("priorities")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|v| v.as_str())
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();
        if requested.len() > 3 {
            return Err(LifeError::Invalid(
                "A daily session records at most three priorities".into(),
            ));
        }
        let priorities = requested;
        self.conn.execute(
            "INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![format!("planning.session.{date}"), json!({"priorities":priorities,"at":now()}).to_string()],
        )?;
        let capacity = self.plan_capacity(&json!({"date":date}))?;
        let commitments = self.rows(
            "SELECT id,title,start_at,end_at,kind,locked FROM events WHERE substr(start_at,1,10)=?1 ORDER BY start_at",
            &[&date],
        )?;
        let tasks = self.rows(
            "SELECT id,title,priority,due_date,estimate_minutes FROM tasks WHERE deleted_at IS NULL AND status NOT IN ('done','cancelled') AND (planned_date=?1 OR due_date=?1) ORDER BY priority ASC LIMIT 20",
            &[&date],
        )?;
        Ok(json!({
            "date":date,
            "priorities":priorities,
            "commitments":commitments,
            "tasks":tasks,
            "capacity_minutes":capacity["capacity_minutes"].clone(),
            "scheduled_minutes":capacity["scheduled_minutes"].clone(),
            "source":"Local SQLite: the day's events and tasks, and the computed availability."
        }))
    }

    /// The recorded priorities of a daily planning session.
    pub fn planning_session_get(&self, p: &Value) -> Result<Value> {
        let date = require(p, "date")?;
        let row = self.rows(
            "SELECT key,value FROM settings WHERE key=?1",
            &[&format!("planning.session.{date}")],
        )?;
        let value = row
            .as_array()
            .and_then(|rows| rows.first())
            .and_then(|r| r["value"].as_str())
            .and_then(|v| serde_json::from_str::<Value>(v).ok())
            .unwrap_or(json!({"priorities":[]}));
        Ok(json!({"date":date,"session":value}))
    }

    // --- capture ---------------------------------------------------------------

    /// Parse natural language into a task draft. The parser is deterministic
    /// and states what it understood.
    pub fn capture_parse(&self, p: &Value) -> Result<Value> {
        let text = require(p, "text")?;
        let lower = text.to_lowercase();
        let mut title = text.trim().to_string();
        let mut due_date = Value::Null;
        let mut priority = 3i64;
        let mut tags: Vec<String> = Vec::new();
        let today = Local::now().date_naive();

        let weekdays = [
            ("monday", 0),
            ("tuesday", 1),
            ("wednesday", 2),
            ("thursday", 3),
            ("friday", 4),
            ("saturday", 5),
            ("sunday", 6),
        ];
        for (name, index) in weekdays {
            if lower.contains(name) {
                let mut date = today;
                let forward = (index + 7 - today.weekday().num_days_from_monday() as i64) % 7;
                date += Duration::days(forward);
                due_date = json!(date.format("%Y-%m-%d").to_string());
                title = title
                    .replace(name, "")
                    .replace(&name.to_uppercase(), "")
                    .trim()
                    .to_string();
                break;
            }
        }
        if due_date.is_null() {
            if lower.contains("tomorrow") {
                due_date = json!((today + Duration::days(1)).format("%Y-%m-%d").to_string());
                title = title.replace("tomorrow", "").trim().to_string();
            } else if lower.contains("today") {
                due_date = json!(today.format("%Y-%m-%d").to_string());
                title = title.replace("today", "").trim().to_string();
            }
        }
        if lower.contains("urgent") || lower.contains("p1") {
            priority = 1;
            title = title
                .replace("urgent", "")
                .replace("p1", "")
                .trim()
                .to_string();
        } else if lower.contains("important") || lower.contains("p2") {
            priority = 2;
            title = title
                .replace("important", "")
                .replace("p2", "")
                .trim()
                .to_string();
        }
        if let Some(open) = title.find('#') {
            let rest = title[open + 1..].to_string();
            let tag: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '-')
                .collect();
            if !tag.is_empty() {
                tags.push(tag.to_lowercase());
                title = format!("{} {}", &title[..open], &title[open + 1 + tag.len()..])
                    .trim()
                    .to_string();
            }
        }
        let title = title
            .trim()
            .trim_matches(|c: char| c == '-' || c == ',' || c == '.')
            .trim()
            .to_string();
        if title.is_empty() {
            return Err(LifeError::Invalid(
                "The capture text did not contain a usable title".into(),
            ));
        }
        Ok(json!({
            "title":title,
            "due_date":due_date,
            "priority":priority,
            "tags":tags,
            "status":"inbox",
            "source":"Deterministic local parser: weekday and today/tomorrow phrases, urgency words and #tags."
        }))
    }

    // --- edge states -----------------------------------------------------------

    /// The database health and the state the shell renders around it.
    pub fn system_status(&self) -> Result<Value> {
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?;
        let integrity: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .unwrap_or_else(|_| "unknown".to_string());
        let tasks: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE deleted_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let bytes = fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
        let journal_mode: String = self
            .conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap_or_else(|_| "unknown".to_string());
        let foreign_keys: i64 = self
            .conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap_or(0);
        Ok(json!({
            "schema_version":version,
            "expected_schema_version":SCHEMA_VERSION,
            "integrity":integrity,
            "journal_mode":journal_mode,
            "foreign_keys":foreign_keys,
            "open_tasks":tasks,
            "database_path":self.path.to_string_lossy(),
            "database_bytes":bytes,
            "up_to_date":version == SCHEMA_VERSION,
            "source":"Local SQLite: the live database pragmas and record counts."
        }))
    }

    /// The storage report used by the diagnostics board.
    pub fn diagnostics_report(&self) -> Result<Value> {
        let status = self.system_status()?;
        let tables = [
            "areas",
            "projects",
            "goals",
            "tasks",
            "events",
            "habits",
            "habit_logs",
            "notes",
            "focus_sessions",
            "reviews",
            "routines",
            "attachments",
        ];
        let mut counts: Vec<Value> = Vec::new();
        for table in tables {
            let sql = match table {
                "attachments" => "SELECT COUNT(*) FROM managed_attachments".to_string(),
                other => format!("SELECT COUNT(*) FROM {other}"),
            };
            let count: i64 = self.conn.query_row(&sql, [], |r| r.get(0)).unwrap_or(0);
            counts.push(json!({"table":table,"rows":count}));
        }
        let audit = self.rows(
            "SELECT event,actor,detail,created_at FROM security_audit ORDER BY created_at DESC LIMIT 20",
            &[],
        )?;
        Ok(json!({
            "status":status,
            "tables":counts,
            "security_audit":audit,
            "source":"Local SQLite: live row counts, database pragmas and the security audit log."
        }))
    }

    /// A security audit entry.
    pub fn security_audit_log(&self, p: &Value, actor: &str) -> Result<Value> {
        let event = require(p, "event")?;
        self.conn.execute(
            "INSERT INTO security_audit(id,event,actor,detail,created_at) VALUES(?1,?2,?3,?4,?5)",
            params![id(), event, actor, text(p, "detail"), now()],
        )?;
        Ok(json!({"event":event,"actor":actor,"detail":text(p, "detail")}))
    }

    pub fn security_audit_list(&self, p: &Value) -> Result<Value> {
        let limit = integer(p, "limit", 100).clamp(1, 500);
        self.rows(
            "SELECT * FROM security_audit ORDER BY created_at DESC LIMIT ?1",
            &[&limit],
        )
    }
}
