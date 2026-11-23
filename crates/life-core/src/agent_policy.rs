//! Agent policy: per-actor action permissions, reviewable proposals and
//! scheduled automation triggers.
use super::*;
use chrono::{Datelike, NaiveTime};

impl Store {
    /// Whether an actor may perform an action.
    ///
    /// The local 'user' actor is always allowed. For any other actor the
    /// permission rows are an allowlist: an actor with no rules is
    /// unrestricted, which is the default state of a fresh workspace. Once an
    /// actor has at least one rule, only actions matching an allowed rule may
    /// run, and a rule with allowed = 0 explicitly denies its pattern.
    pub fn actor_permitted(&self, actor: &str, action: &str) -> Result<bool> {
        if actor == "user" || actor.is_empty() {
            return Ok(true);
        }
        // Reading the permission model itself is always permitted, otherwise
        // an operator could not see why an action was refused.
        if action.starts_with("agents.permissions.list") {
            return Ok(true);
        }
        let rows = self.rows(
            "SELECT action_pattern,allowed FROM agent_permissions WHERE actor=?1",
            &[&actor],
        )?;
        let rules = rows.as_array().cloned().unwrap_or_default();
        if rules.is_empty() {
            return Ok(true);
        }
        let mut allowed = false;
        let mut matched = false;
        for rule in &rules {
            let pattern = rule["action_pattern"].as_str().unwrap_or("");
            let hit = if pattern == "*" {
                true
            } else if let Some(prefix) = pattern.strip_suffix(".*") {
                action == prefix || action.starts_with(&format!("{prefix}."))
            } else {
                pattern == action
            };
            if hit {
                matched = true;
                allowed = rule["allowed"].as_i64().unwrap_or(0) == 1;
            }
        }
        if !matched {
            return Ok(false);
        }
        Ok(allowed)
    }

    pub fn agent_permissions_list(&self, p: &Value) -> Result<Value> {
        let actor = optional(p, "actor");
        match actor {
            Some(a) => self.rows(
                "SELECT * FROM agent_permissions WHERE actor=?1 ORDER BY action_pattern",
                &[&a],
            ),
            None => self.rows(
                "SELECT * FROM agent_permissions ORDER BY actor,action_pattern",
                &[],
            ),
        }
    }

    pub fn agent_permissions_set(&self, p: &Value, actor: &str) -> Result<Value> {
        let target = require(p, "target_actor")?;
        let pattern = require(p, "action_pattern")?;
        let allowed = p.get("allowed").and_then(Value::as_bool).unwrap_or(true);
        self.conn.execute(
            "INSERT INTO agent_permissions(actor,action_pattern,allowed,created_at) VALUES(?1,?2,?3,?4) ON CONFLICT(actor,action_pattern) DO UPDATE SET allowed=excluded.allowed",
            params![target, pattern, i64::from(allowed), now()],
        )?;
        self.record(actor, "agents.permissions.set", Some(&target), p)?;
        self.rows(
            "SELECT * FROM agent_permissions WHERE actor=?1 AND action_pattern=?2",
            &[&target, &pattern],
        )
        .map(|v| v[0].clone())
    }

    pub fn agent_permissions_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let target = require(p, "target_actor")?;
        let pattern = require(p, "action_pattern")?;
        let n = self.conn.execute(
            "DELETE FROM agent_permissions WHERE actor=?1 AND action_pattern=?2",
            [&target, &pattern],
        )?;
        if n == 0 {
            return Err(LifeError::NotFound(format!("{target}:{pattern}")));
        }
        self.record(actor, "agents.permissions.delete", Some(&target), p)?;
        Ok(json!({"deleted":true,"actor":target,"action_pattern":pattern}))
    }

    pub fn agent_proposals_list(&self, p: &Value) -> Result<Value> {
        let state = optional(p, "state");
        match state {
            Some(s) => self.rows(
                "SELECT * FROM agent_proposals WHERE state=?1 ORDER BY created_at DESC",
                &[&s],
            ),
            None => self.rows(
                "SELECT * FROM agent_proposals ORDER BY created_at DESC LIMIT 200",
                &[],
            ),
        }
    }

    /// Record a proposed mutation for human review.
    pub fn agent_proposals_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let action = require(p, "action")?;
        let payload = p.get("payload").cloned().unwrap_or_else(|| json!({}));
        if !payload.is_object() {
            return Err(LifeError::Invalid("payload must be an object".into()));
        }
        let key = id();
        self.conn.execute(
            "INSERT INTO agent_proposals(id,actor,action,payload_json,state,created_at) VALUES(?1,?2,?3,?4,'pending',?5)",
            params![key, actor, action, payload.to_string(), now()],
        )?;
        self.record(actor, "agents.proposals.create", Some(&key), p)?;
        self.rows("SELECT * FROM agent_proposals WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }

    /// Approve a proposal and apply the recorded action.
    pub fn agent_proposals_approve(&mut self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let row = self
            .rows("SELECT * FROM agent_proposals WHERE id=?1", &[&key])?
            .as_array()
            .and_then(|v| v.first())
            .cloned()
            .ok_or_else(|| LifeError::NotFound(key.clone()))?;
        if row["state"].as_str() != Some("pending") {
            return Err(LifeError::Conflict("Proposal is not pending".into()));
        }
        let action = row["action"].as_str().unwrap_or("").to_string();
        let payload: Value = serde_json::from_str(row["payload_json"].as_str().unwrap_or("{}"))
            .map_err(|_| LifeError::Invalid("Stored proposal payload is not valid JSON".into()))?;
        let mut applied = payload.as_object().cloned().unwrap_or_default();
        applied.insert("actor".into(), json!(actor));
        let result = self.call(&action, &Value::Object(applied))?;
        self.conn.execute(
            "UPDATE agent_proposals SET state='approved',decision_reason=?2,decided_at=?3 WHERE id=?1",
            params![key, text(p, "reason"), now()],
        )?;
        Ok(json!({"id":key,"state":"approved","action":action,"result":result}))
    }

    pub fn agent_proposals_reject(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let row = self
            .rows("SELECT * FROM agent_proposals WHERE id=?1", &[&key])?
            .as_array()
            .and_then(|v| v.first())
            .cloned()
            .ok_or_else(|| LifeError::NotFound(key.clone()))?;
        if row["state"].as_str() != Some("pending") {
            return Err(LifeError::Conflict("Proposal is not pending".into()));
        }
        let reason = require(p, "reason")?;
        self.conn.execute(
            "UPDATE agent_proposals SET state='rejected',decision_reason=?2,decided_at=?3 WHERE id=?1",
            params![key, reason, now()],
        )?;
        self.record(actor, "agents.proposals.reject", Some(&key), p)?;
        Ok(json!({"id":key,"state":"rejected","reason":reason}))
    }

    pub fn automation_schedules_list(&self, p: &Value) -> Result<Value> {
        let rule = optional(p, "rule_id");
        match rule {
            Some(r) => self.rows(
                "SELECT * FROM automation_schedules WHERE rule_id=?1 ORDER BY created_at",
                &[&r],
            ),
            None => self.rows(
                "SELECT * FROM automation_schedules ORDER BY created_at DESC",
                &[],
            ),
        }
    }

    pub fn automation_schedules_set(&self, p: &Value, actor: &str) -> Result<Value> {
        let rule = require(p, "rule_id")?;
        let found = self
            .rows("SELECT id FROM automation_rules WHERE id=?1", &[&rule])?
            .as_array()
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        if !found {
            return Err(LifeError::NotFound(rule));
        }
        let kind = optional(p, "kind").unwrap_or_else(|| "daily".into());
        if !["daily", "weekly"].contains(&kind.as_str()) {
            return Err(LifeError::Invalid("kind must be daily or weekly".into()));
        }
        let at_time = require(p, "at_time")?;
        NaiveTime::parse_from_str(&at_time, "%H:%M")
            .map_err(|_| LifeError::Invalid("at_time must be HH:MM".into()))?;
        let weekday = p.get("weekday").and_then(Value::as_i64);
        if kind == "weekly" && weekday.is_none() {
            return Err(LifeError::Invalid(
                "weekday is required for a weekly schedule".into(),
            ));
        }
        if let Some(day) = weekday
            && !(0..=6).contains(&day)
        {
            return Err(LifeError::Invalid(
                "weekday must be 0 (Monday) through 6 (Sunday)".into(),
            ));
        }
        let key = id();
        self.conn.execute(
            "INSERT INTO automation_schedules(id,rule_id,kind,at_time,weekday,enabled,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![key, rule, kind, at_time, weekday, i64::from(p.get("enabled").and_then(Value::as_bool).unwrap_or(true)), now()],
        )?;
        self.record(actor, "automations.schedule.set", Some(&key), p)?;
        self.rows("SELECT * FROM automation_schedules WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }

    pub fn automation_schedules_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let n = self
            .conn
            .execute("DELETE FROM automation_schedules WHERE id=?1", [&key])?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "automations.schedule.delete", Some(&key), p)?;
        Ok(json!({"deleted":true,"id":key}))
    }

    /// Schedules that are due right now and have not run today.
    pub fn automation_schedules_due(&self, _p: &Value) -> Result<Value> {
        let now = Local::now();
        let today = now.date_naive();
        let weekday = i64::from(today.weekday().num_days_from_monday());
        let time = now.time().format("%H:%M").to_string();
        let day = today.format("%Y-%m-%d").to_string();
        let rows = self.rows(
            "SELECT s.*,r.title AS rule_title,r.action AS rule_action,r.arguments AS rule_arguments,r.enabled AS rule_enabled FROM automation_schedules s JOIN automation_rules r ON r.id=s.rule_id WHERE s.enabled=1 AND r.enabled=1 AND (s.kind='daily' OR s.weekday=?1)",
            &[&weekday],
        )?;
        let mut due: Vec<Value> = Vec::new();
        for schedule in rows.as_array().into_iter().flatten() {
            let at = schedule["at_time"].as_str().unwrap_or("");
            if at > time.as_str() {
                continue;
            }
            let last = schedule["last_run"].as_str().unwrap_or("");
            if last.starts_with(day.as_str()) {
                continue;
            }
            due.push(schedule.clone());
        }
        let count = due.len();
        Ok(json!({"due":due,"count":count,"checked_at":now.to_rfc3339()}))
    }

    /// Run every due schedule once and record the result.
    pub fn automation_schedules_run_due(&mut self, p: &Value, actor: &str) -> Result<Value> {
        let due = self.automation_schedules_due(p)?;
        let mut ran = Vec::new();
        for schedule in due["due"].as_array().into_iter().flatten() {
            let id = schedule["id"].as_str().unwrap_or("").to_string();
            let rule_id = schedule["rule_id"].as_str().unwrap_or("").to_string();
            let arguments = schedule["rule_arguments"]
                .as_str()
                .unwrap_or("{}")
                .to_string();
            let action = schedule["rule_action"].as_str().unwrap_or("").to_string();
            let payload: Value = serde_json::from_str(&arguments).unwrap_or_else(|_| json!({}));
            let mut call = payload.as_object().cloned().unwrap_or_default();
            call.insert("actor".into(), json!(actor));
            let outcome = self.call(&action, &Value::Object(call));
            let ok = outcome.is_ok();
            self.conn.execute(
                "UPDATE automation_schedules SET last_run=?2 WHERE id=?1",
                params![id, now()],
            )?;
            ran.push(json!({
                "schedule_id":id,
                "rule_id":rule_id,
                "action":action,
                "ok":ok,
                "error": outcome.err().map(|e| e.to_string()),
            }));
        }
        let count = ran.len();
        Ok(json!({"ran":ran,"count":count}))
    }
}
