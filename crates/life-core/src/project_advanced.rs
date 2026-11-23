//! FigCraft project templates, blockers, and explainable health diagnostics.
//! All writes run inside the Store::call transaction.
use super::*;

impl Store {
    pub fn project_health(&self, p: &Value) -> Result<Value> {
        let project_id = require(p, "id")?;
        let project = self.fetch("projects", &project_id)?;
        let tasks = self.rows(
            "SELECT t.id,t.title,t.status,t.priority,t.due_date,t.estimate_minutes,
                EXISTS(SELECT 1 FROM task_dependencies d JOIN tasks upstream ON upstream.id=d.task_id
                       WHERE d.blocks_id=t.id AND upstream.deleted_at IS NULL AND upstream.status!='done') AS blocked
             FROM tasks t WHERE t.project_id=?1 AND t.deleted_at IS NULL
             ORDER BY t.priority,t.due_date,t.title",
            &[&project_id],
        )?;
        let milestones = self.rows(
            "SELECT id,title,status,due_date FROM milestones WHERE project_id=?1 ORDER BY due_date,title",
            &[&project_id],
        )?;
        let today = Local::now().date_naive().to_string();
        let mut open = 0;
        let mut completed = 0;
        let mut overdue = 0;
        let mut blocked = 0;
        let mut remaining_minutes = 0;
        let mut issues = Vec::new();
        for t in tasks.as_array().into_iter().flatten() {
            if t["status"] == "done" {
                completed += 1;
                continue;
            }
            if t["status"] == "cancelled" {
                continue;
            }
            open += 1;
            remaining_minutes += t["estimate_minutes"].as_i64().unwrap_or(0).max(0);
            if t["blocked"].as_i64() == Some(1) {
                blocked += 1;
                issues.push(
                    json!({"kind":"blocked_task","id":t["id"],"title":t["title"],
                    "reason":"An unfinished predecessor must be completed"}),
                );
            }
            if let Some(date) = t["due_date"].as_str()
                && date < today.as_str()
            {
                overdue += 1;
                issues.push(
                    json!({"kind":"overdue_task","id":t["id"],"title":t["title"],
                    "due_date":date}),
                );
            }
        }
        let mut overdue_milestones = 0;
        for m in milestones.as_array().into_iter().flatten() {
            if m["status"] != "done"
                && let Some(date) = m["due_date"].as_str()
                && date < today.as_str()
            {
                overdue_milestones += 1;
                issues.push(
                    json!({"kind":"overdue_milestone","id":m["id"],"title":m["title"],
                    "due_date":date}),
                );
            }
        }
        let total = open + completed;
        let completion_percent = if total == 0 {
            0
        } else {
            completed * 100 / total
        };
        let health = if overdue > 0 || overdue_milestones > 0 {
            "needs_attention"
        } else if blocked > 0 {
            "blocked"
        } else if open == 0 && total > 0 {
            "complete"
        } else {
            "on_track"
        };
        Ok(json!({
            "project":project, "health":health, "as_of":today,
            "tasks":{"open":open,"completed":completed,"total":total,"overdue":overdue,
                     "blocked":blocked,"remaining_estimate_minutes":remaining_minutes,
                     "completion_percent":completion_percent},
            "milestones":{"total":milestones.as_array().map_or(0,Vec::len),
                "overdue":overdue_milestones},
            "issues":issues,
            "methodology":"Open/completed tasks and incomplete dependency edges come directly from SQLite. \
                           Overdue means due_date earlier than local today. Status is deterministic \
                           and does not predict delivery dates."
        }))
    }
    pub fn project_templates_list(&self) -> Result<Value> {
        self.rows(
            "SELECT id,name,description,created_at,updated_at FROM project_templates
             WHERE archived_at IS NULL ORDER BY updated_at DESC",
            &[],
        )
    }
    pub fn project_templates_get(&self, p: &Value) -> Result<Value> {
        let key = require(p, "id")?;
        let result = self.rows(
            "SELECT * FROM project_templates WHERE id=?1 AND archived_at IS NULL",
            &[&key],
        )?;
        result
            .as_array()
            .and_then(|v| v.first())
            .cloned()
            .ok_or(LifeError::NotFound(key))
    }
    pub fn project_templates_capture(&self, p: &Value, actor: &str) -> Result<Value> {
        let project_id = require(p, "project_id")?;
        let project = self.fetch("projects", &project_id)?;
        let name = optional(p, "name")
            .unwrap_or_else(|| project["name"].as_str().unwrap_or("Project").to_string());
        if name.trim().is_empty() || name.chars().count() > 180 {
            return Err(LifeError::Invalid(
                "Template name must be 1–180 characters".into(),
            ));
        }
        let tasks=self.rows(
            "SELECT id,title,description,priority,estimate_minutes,parent_id
             FROM tasks WHERE project_id=?1 AND deleted_at IS NULL ORDER BY created_at,id LIMIT 250",
            &[&project_id],
        )?;
        let milestones = self.rows(
            "SELECT title FROM milestones WHERE project_id=?1 ORDER BY created_at,id LIMIT 100",
            &[&project_id],
        )?;
        let task_records = tasks.as_array().cloned().unwrap_or_default();
        let task_ids: std::collections::HashSet<&str> = task_records
            .iter()
            .filter_map(|t| t["id"].as_str())
            .collect();
        let edges = self.rows(
            "SELECT task_id,blocks_id FROM task_dependencies WHERE
             task_id IN (SELECT id FROM tasks WHERE project_id=?1 AND deleted_at IS NULL)
             AND blocks_id IN (SELECT id FROM tasks WHERE project_id=?1 AND deleted_at IS NULL)",
            &[&project_id],
        )?;
        // Do not bake absolute dates, IDs or life-area associations into reusable templates.
        let clean: Vec<Value> = task_records
            .iter()
            .map(|t| {
                json!({
                    "source_id":t["id"],"title":t["title"],"description":t["description"],
                    "priority":t["priority"],"estimate_minutes":t["estimate_minutes"],
                    "parent_source_id":t["parent_id"].as_str().filter(|v|task_ids.contains(v))
                })
            })
            .collect();
        let snapshot = json!({
            "version":1,"description":project["description"],"tasks":clean,
            "milestones":milestones.as_array().unwrap_or(&Vec::new()).iter()
               .map(|m|json!({"title":m["title"]})).collect::<Vec<_>>(),
            "dependencies":edges
        });
        let key = id();
        let timestamp = now();
        self.conn.execute(
            "INSERT INTO project_templates(id,name,description,snapshot_json,created_at,updated_at)
             VALUES(?1,?2,?3,?4,?5,?5)",
            params![
                key,
                name,
                text(p, "description"),
                snapshot.to_string(),
                timestamp
            ],
        )?;
        self.record(actor, "projects.template.capture", Some(&key), p)?;
        self.project_templates_get(&json!({"id":key}))
    }
    pub fn project_templates_apply(&self, p: &Value, actor: &str) -> Result<Value> {
        let template = self.project_templates_get(p)?;
        let snapshot: Value =
            serde_json::from_str(template["snapshot_json"].as_str().unwrap_or("{}"))?;
        if snapshot["version"] != 1 {
            return Err(LifeError::Invalid(
                "Unsupported project template version".into(),
            ));
        }
        let name = optional(p, "name")
            .unwrap_or_else(|| template["name"].as_str().unwrap_or("Project").to_string());
        if name.trim().is_empty() || name.chars().count() > 180 {
            return Err(LifeError::Invalid("Invalid project name".into()));
        }
        let project = self.create_project(
            &json!({
                "name":name,"description":snapshot["description"],"area_id":p["area_id"]
            }),
            actor,
        )?;
        let project_id = require(&project, "id")?;
        let mut ids = std::collections::HashMap::<String, String>::new();
        let template_tasks = snapshot["tasks"]
            .as_array()
            .ok_or_else(|| LifeError::Invalid("Missing template tasks".into()))?;
        if template_tasks.len() > 250 {
            return Err(LifeError::Invalid(
                "Project template task cap exceeded".into(),
            ));
        }
        let mut new_tasks = Vec::new();
        for t in template_tasks {
            let task = self.create_task(
                &json!({
                    "title":t["title"],"description":t["description"],"priority":t["priority"],
                    "estimate_minutes":t["estimate_minutes"],"project_id":project_id
                }),
                actor,
            )?;
            ids.insert(require(t, "source_id")?, require(&task, "id")?);
            new_tasks.push(task);
        }
        // Parent links are resolved only after all tasks exist.
        for t in template_tasks {
            if let Some(parent) = t["parent_source_id"].as_str()
                && let (Some(key), Some(parent_key)) =
                    (ids.get(&require(t, "source_id")?), ids.get(parent))
            {
                self.update_task(&json!({"id":key,"parent_id":parent_key}), actor)?;
            }
        }
        for edge in snapshot["dependencies"].as_array().into_iter().flatten() {
            if let (Some(task), Some(blocks)) = (
                edge["task_id"].as_str().and_then(|v| ids.get(v)),
                edge["blocks_id"].as_str().and_then(|v| ids.get(v)),
            ) {
                self.add_dependency(&json!({"task_id":task,"blocks_id":blocks}), actor)?;
            }
        }
        let template_milestones = snapshot["milestones"]
            .as_array()
            .ok_or_else(|| LifeError::Invalid("Missing milestones".into()))?;
        if template_milestones.len() > 100 {
            return Err(LifeError::Invalid("Milestone cap exceeded".into()));
        }
        let mut created_milestones = Vec::new();
        for m in template_milestones {
            created_milestones.push(self.create_milestone(
                &json!({
                    "project_id":project_id,"title":m["title"]
                }),
                actor,
            )?);
        }
        self.record(
            actor,
            "projects.template.apply",
            Some(&project_id),
            &json!({"template_id":template["id"],"project_id":project_id}),
        )?;
        Ok(json!({"project":project,"tasks":new_tasks,"milestones":created_milestones}))
    }
    pub fn project_templates_archive(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let n = self.conn.execute(
            "UPDATE project_templates SET archived_at=?2,updated_at=?2
             WHERE id=?1 AND archived_at IS NULL",
            params![key, now()],
        )?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "projects.template.archive", Some(&key), p)?;
        Ok(json!({"archived":true,"id":key}))
    }
}
