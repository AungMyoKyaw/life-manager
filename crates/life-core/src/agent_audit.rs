//! Agent-visible mutation evidence and guarded task undo.
use super::*;
impl Store {
    pub fn task_snapshot(&self, key: &str) -> Result<Option<Value>> {
        Ok(self
            .rows("SELECT * FROM tasks WHERE id=?1", &[&key])?
            .as_array()
            .and_then(|v| v.first())
            .cloned())
    }
    pub fn write_change(
        &self,
        action: &str,
        actor: &str,
        request_id: Option<&str>,
        key: &str,
        before: Option<Value>,
        after: Option<Value>,
    ) -> Result<()> {
        if ![
            "tasks.create",
            "tasks.update",
            "tasks.delete",
            "tasks.snooze",
        ]
        .contains(&action)
        {
            return Ok(());
        }
        let undoable = if after.is_some() { 1 } else { 0 };
        self.conn.execute("INSERT INTO action_changes(id,actor,action,entity_id,request_id,before_json,after_json,undoable,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![id(),actor,action,key,request_id,before.map(|v|v.to_string()),after.map(|v|v.to_string()),undoable,now()])?;
        Ok(())
    }
    pub fn agent_changes(&self) -> Result<Value> {
        self.rows("SELECT id,actor,action,entity_id,request_id,before_json,after_json,undoable,undone_at,created_at FROM action_changes ORDER BY created_at DESC,id DESC LIMIT 100",&[])
    }
    pub fn agent_undo(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let rows = self.rows("SELECT * FROM action_changes WHERE id=?1", &[&key])?;
        let change = rows
            .as_array()
            .and_then(|v| v.first())
            .ok_or_else(|| LifeError::NotFound(key.clone()))?;
        if change["undoable"].as_i64() != Some(1) || !change["undone_at"].is_null() {
            return Err(LifeError::Conflict(
                "Change is not eligible for undo".into(),
            ));
        }
        let entity_id = require(change, "entity_id")?;
        let action = require(change, "action")?;
        let after: Value = serde_json::from_str(
            change["after_json"]
                .as_str()
                .ok_or_else(|| LifeError::Conflict("Missing after-snapshot".into()))?,
        )?;
        let actual = self
            .task_snapshot(&entity_id)?
            .ok_or_else(|| LifeError::Conflict("Target task no longer exists".into()))?;
        if actual["revision"] != after["revision"] || actual["deleted_at"] != after["deleted_at"] {
            return Err(LifeError::Conflict(
                "Task has changed since that mutation; undo would overwrite newer work".into(),
            ));
        }
        if action == "tasks.create" {
            self.conn.execute(
                "UPDATE tasks SET deleted_at=?2,updated_at=?2,revision=revision+1 WHERE id=?1",
                params![entity_id, now()],
            )?;
        } else {
            let before: Value = serde_json::from_str(
                change["before_json"]
                    .as_str()
                    .ok_or_else(|| LifeError::Conflict("Missing before-snapshot".into()))?,
            )?;
            self.conn.execute("UPDATE tasks SET title=?2,description=?3,status=?4,priority=?5,project_id=?6,area_id=?7,parent_id=?8,goal_id=?9,due_date=?10,planned_date=?11,estimate_minutes=?12,recurrence=?13,tags=?14,completed_at=?15,deleted_at=?16,updated_at=?17,revision=revision+1 WHERE id=?1",params![
                entity_id,
                before["title"].as_str(),before["description"].as_str(),before["status"].as_str(),before["priority"].as_i64(),
                before["project_id"].as_str(),before["area_id"].as_str(),before["parent_id"].as_str(),before["goal_id"].as_str(),
                before["due_date"].as_str(),before["planned_date"].as_str(),before["estimate_minutes"].as_i64(),before["recurrence"].as_str(),before["tags"].as_str(),
                before["completed_at"].as_str(),before["deleted_at"].as_str(),now()
            ])?;
        }
        self.conn.execute(
            "UPDATE action_changes SET undone_at=?2 WHERE id=?1",
            params![key, now()],
        )?;
        self.record(
            actor,
            "agents.undo",
            Some(&entity_id),
            &json!({"change_id":key,"original_action":action}),
        )?;
        Ok(
            json!({"change_id":key,"entity_id":entity_id,"undone":true,"task":self.task_snapshot(&entity_id)?}),
        )
    }
}
