//! The remaining personal life-area boards: interviews, savings goals,
//! certifications, rest intentions and personal administration.
use super::*;

fn require_day(p: &Value, field: &str) -> Result<Option<String>> {
    let value = optional(p, field);
    if let Some(day) = &value {
        NaiveDate::parse_from_str(day, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid(format!("{field} must be YYYY-MM-DD")))?;
    }
    Ok(value)
}

fn money(value: &Value, field: &str) -> Result<i64> {
    // An absent amount is zero. A negative amount is a mistake.
    let amount = match value.get(field) {
        None => 0,
        Some(inner) => inner.as_i64().ok_or_else(|| {
            LifeError::Invalid(format!("{field} must be an integer of minor units"))
        })?,
    };
    if amount < 0 {
        return Err(LifeError::Invalid(format!(
            "{field} must be zero or a positive integer of minor units"
        )));
    }
    Ok(amount)
}

impl Store {
    // --- interviews ---------------------------------------------------------

    pub fn interviews_list(&self, p: &Value) -> Result<Value> {
        let application = optional(p, "application_id");
        match application {
            Some(id) => self.rows(
                "SELECT i.*,c.employer,c.role FROM interview_events i LEFT JOIN career_applications c ON c.id=i.application_id WHERE i.deleted_at IS NULL AND i.application_id=?1 ORDER BY i.starts_at",
                &[&id],
            ),
            None => self.rows(
                "SELECT i.*,c.employer,c.role FROM interview_events i LEFT JOIN career_applications c ON c.id=i.application_id WHERE i.deleted_at IS NULL ORDER BY i.starts_at",
                &[],
            ),
        }
    }

    pub fn interviews_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let title = require(p, "title")?;
        let starts_at = require(p, "starts_at")?;
        let ends_at = optional(p, "ends_at");
        if let Some(end) = &ends_at
            && end <= &starts_at
        {
            return Err(LifeError::Invalid("ends_at must be after starts_at".into()));
        }
        let application = optional(p, "application_id");
        if let Some(id) = &application {
            self.fetch("career_applications", id)?;
        }
        let key = id();
        self.conn.execute(
            "INSERT INTO interview_events(id,application_id,title,kind,starts_at,ends_at,location,preparation,status,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10)",
            params![
                key,
                application,
                title,
                optional(p, "kind").unwrap_or_else(|| "interview".into()),
                starts_at,
                ends_at,
                text(p, "location"),
                text(p, "preparation"),
                optional(p, "status").unwrap_or_else(|| "scheduled".into()),
                now()
            ],
        )?;
        self.record(actor, "interviews.create", Some(&key), p)?;
        self.rows("SELECT * FROM interview_events WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }

    pub fn interviews_update(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let current = self
            .rows(
                "SELECT * FROM interview_events WHERE id=?1 AND deleted_at IS NULL",
                &[&key],
            )?
            .as_array()
            .and_then(|v| v.first())
            .cloned()
            .ok_or_else(|| LifeError::NotFound(key.clone()))?;
        let get = |k: &str| p.get(k).cloned().unwrap_or_else(|| current[k].clone());
        let starts_at = get("starts_at").as_str().unwrap_or("").to_string();
        let ends_at = optional(p, "ends_at").or_else(|| {
            current["ends_at"]
                .as_str()
                .filter(|v| !v.is_empty())
                .map(|v| v.to_string())
        });
        if let Some(end) = &ends_at
            && end <= &starts_at
        {
            return Err(LifeError::Invalid("ends_at must be after starts_at".into()));
        }
        self.conn.execute(
            "UPDATE interview_events SET title=?2,kind=?3,starts_at=?4,ends_at=?5,location=?6,preparation=?7,status=?8,updated_at=?9,revision=revision+1 WHERE id=?1",
            params![
                key,
                get("title").as_str(),
                get("kind").as_str(),
                starts_at,
                ends_at,
                get("location").as_str(),
                get("preparation").as_str(),
                get("status").as_str(),
                now()
            ],
        )?;
        self.record(actor, "interviews.update", Some(&key), p)?;
        self.rows("SELECT * FROM interview_events WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }

    pub fn interviews_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let n = self.conn.execute(
            "UPDATE interview_events SET deleted_at=?2,revision=revision+1 WHERE id=?1 AND deleted_at IS NULL",
            params![key, now()],
        )?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "interviews.delete", Some(&key), p)?;
        Ok(json!({"archived":true,"id":key}))
    }

    // --- savings goals ------------------------------------------------------

    pub fn savings_goals_list(&self) -> Result<Value> {
        self.rows(
            "SELECT * FROM savings_goals WHERE deleted_at IS NULL ORDER BY COALESCE(due_date,'9999-12-31')",
            &[],
        )
    }

    pub fn savings_goals_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let title = require(p, "title")?;
        let target = money(p, "target_minor")?;
        let saved = money(p, "saved_minor")?;
        require_day(p, "due_date")?;
        let key = id();
        self.conn.execute(
            "INSERT INTO savings_goals(id,title,target_minor,saved_minor,currency,due_date,note,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?8)",
            params![
                key,
                title,
                target,
                saved,
                optional(p, "currency").unwrap_or_else(|| "SGD".into()),
                optional(p, "due_date"),
                text(p, "note"),
                now()
            ],
        )?;
        self.record(actor, "savings_goals.create", Some(&key), p)?;
        self.rows("SELECT * FROM savings_goals WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }

    pub fn savings_goals_update(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let current = self
            .rows(
                "SELECT * FROM savings_goals WHERE id=?1 AND deleted_at IS NULL",
                &[&key],
            )?
            .as_array()
            .and_then(|v| v.first())
            .cloned()
            .ok_or_else(|| LifeError::NotFound(key.clone()))?;
        let get = |k: &str| p.get(k).cloned().unwrap_or_else(|| current[k].clone());
        if p.get("target_minor").is_some() {
            money(p, "target_minor")?;
        }
        if p.get("saved_minor").is_some() {
            money(p, "saved_minor")?;
        }
        require_day(p, "due_date")?;
        self.conn.execute(
            "UPDATE savings_goals SET title=?2,target_minor=?3,saved_minor=?4,currency=?5,due_date=?6,note=?7,updated_at=?8,revision=revision+1 WHERE id=?1",
            params![
                key,
                get("title").as_str(),
                get("target_minor").as_i64(),
                get("saved_minor").as_i64(),
                get("currency").as_str(),
                get("due_date").as_str(),
                get("note").as_str(),
                now()
            ],
        )?;
        self.record(actor, "savings_goals.update", Some(&key), p)?;
        self.rows("SELECT * FROM savings_goals WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }

    pub fn savings_goals_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let n = self.conn.execute(
            "UPDATE savings_goals SET deleted_at=?2,revision=revision+1 WHERE id=?1 AND deleted_at IS NULL",
            params![key, now()],
        )?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "savings_goals.delete", Some(&key), p)?;
        Ok(json!({"archived":true,"id":key}))
    }

    // --- certifications -----------------------------------------------------

    pub fn certifications_list(&self) -> Result<Value> {
        self.rows(
            "SELECT * FROM certifications WHERE deleted_at IS NULL ORDER BY COALESCE(deadline_date,'9999-12-31')",
            &[],
        )
    }

    pub fn certifications_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let title = require(p, "title")?;
        require_day(p, "exam_date")?;
        require_day(p, "deadline_date")?;
        let target = p
            .get("study_hours_target")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        if target < 0.0 {
            return Err(LifeError::Invalid(
                "study_hours_target cannot be negative".into(),
            ));
        }
        let key = id();
        self.conn.execute(
            "INSERT INTO certifications(id,title,provider,status,exam_date,deadline_date,study_hours_target,study_hours_done,note,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10)",
            params![
                key,
                title,
                text(p, "provider"),
                optional(p, "status").unwrap_or_else(|| "planned".into()),
                optional(p, "exam_date"),
                optional(p, "deadline_date"),
                target,
                p.get("study_hours_done")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                text(p, "note"),
                now()
            ],
        )?;
        self.record(actor, "certifications.create", Some(&key), p)?;
        self.rows("SELECT * FROM certifications WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }

    pub fn certifications_update(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let current = self
            .rows(
                "SELECT * FROM certifications WHERE id=?1 AND deleted_at IS NULL",
                &[&key],
            )?
            .as_array()
            .and_then(|v| v.first())
            .cloned()
            .ok_or_else(|| LifeError::NotFound(key.clone()))?;
        let get = |k: &str| p.get(k).cloned().unwrap_or_else(|| current[k].clone());
        require_day(p, "exam_date")?;
        require_day(p, "deadline_date")?;
        self.conn.execute(
            "UPDATE certifications SET title=?2,provider=?3,status=?4,exam_date=?5,deadline_date=?6,study_hours_target=?7,study_hours_done=?8,note=?9,updated_at=?10,revision=revision+1 WHERE id=?1",
            params![
                key,
                get("title").as_str(),
                get("provider").as_str(),
                get("status").as_str(),
                get("exam_date").as_str(),
                get("deadline_date").as_str(),
                get("study_hours_target").as_f64().unwrap_or(0.0),
                get("study_hours_done").as_f64().unwrap_or(0.0),
                get("note").as_str(),
                now()
            ],
        )?;
        self.record(actor, "certifications.update", Some(&key), p)?;
        self.rows("SELECT * FROM certifications WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }

    pub fn certifications_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let n = self.conn.execute(
            "UPDATE certifications SET deleted_at=?2,revision=revision+1 WHERE id=?1 AND deleted_at IS NULL",
            params![key, now()],
        )?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "certifications.delete", Some(&key), p)?;
        Ok(json!({"archived":true,"id":key}))
    }

    // --- rest intentions ----------------------------------------------------

    pub fn rest_intentions_list(&self) -> Result<Value> {
        self.rows(
            "SELECT * FROM rest_intentions WHERE deleted_at IS NULL ORDER BY kind,title",
            &[],
        )
    }

    pub fn rest_intentions_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let title = require(p, "title")?;
        let kind = optional(p, "kind").unwrap_or_else(|| "sleep".into());
        if !["sleep", "rest", "recovery"].contains(&kind.as_str()) {
            return Err(LifeError::Invalid(
                "kind must be sleep, rest or recovery".into(),
            ));
        }
        let days = p.get("days").cloned().unwrap_or_else(|| json!([]));
        if !days.is_array() {
            return Err(LifeError::Invalid("days must be an array".into()));
        }
        for day in days.as_array().into_iter().flatten() {
            let index = day.as_i64().unwrap_or(-1);
            if !(0..=6).contains(&index) {
                return Err(LifeError::Invalid(
                    "days must be weekday numbers 0 through 6".into(),
                ));
            }
        }
        let key = id();
        self.conn.execute(
            "INSERT INTO rest_intentions(id,title,kind,starts_at,ends_at,days,note,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?8)",
            params![
                key,
                title,
                kind,
                optional(p, "starts_at").unwrap_or_default(),
                optional(p, "ends_at").unwrap_or_default(),
                days.to_string(),
                text(p, "note"),
                now()
            ],
        )?;
        self.record(actor, "rest_intentions.create", Some(&key), p)?;
        self.rows("SELECT * FROM rest_intentions WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }

    pub fn rest_intentions_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let n = self.conn.execute(
            "UPDATE rest_intentions SET deleted_at=?2,revision=revision+1 WHERE id=?1 AND deleted_at IS NULL",
            params![key, now()],
        )?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "rest_intentions.delete", Some(&key), p)?;
        Ok(json!({"archived":true,"id":key}))
    }

    // --- personal administration --------------------------------------------

    pub fn administration_list(&self) -> Result<Value> {
        self.rows(
            "SELECT * FROM administration_items WHERE deleted_at IS NULL ORDER BY COALESCE(renewal_date,'9999-12-31')",
            &[],
        )
    }

    pub fn administration_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let title = require(p, "title")?;
        require_day(p, "renewal_date")?;
        let reminder = integer(p, "reminder_days", 14);
        if !(0..=365).contains(&reminder) {
            return Err(LifeError::Invalid(
                "reminder_days must be 0 through 365".into(),
            ));
        }
        if let Some(amount) = p.get("amount_minor").and_then(Value::as_i64)
            && amount < 0
        {
            return Err(LifeError::Invalid(
                "amount_minor must be zero or a positive integer".into(),
            ));
        }
        let key = id();
        self.conn.execute(
            "INSERT INTO administration_items(id,title,kind,renewal_date,reminder_days,amount_minor,currency,status,note,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10)",
            params![
                key,
                title,
                optional(p, "kind").unwrap_or_else(|| "paperwork".into()),
                optional(p, "renewal_date"),
                reminder,
                p.get("amount_minor").and_then(Value::as_i64),
                optional(p, "currency").unwrap_or_else(|| "SGD".into()),
                optional(p, "status").unwrap_or_else(|| "open".into()),
                text(p, "note"),
                now()
            ],
        )?;
        self.record(actor, "administration.create", Some(&key), p)?;
        self.rows("SELECT * FROM administration_items WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }

    pub fn administration_update(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let current = self
            .rows(
                "SELECT * FROM administration_items WHERE id=?1 AND deleted_at IS NULL",
                &[&key],
            )?
            .as_array()
            .and_then(|v| v.first())
            .cloned()
            .ok_or_else(|| LifeError::NotFound(key.clone()))?;
        let get = |k: &str| p.get(k).cloned().unwrap_or_else(|| current[k].clone());
        require_day(p, "renewal_date")?;
        self.conn.execute(
            "UPDATE administration_items SET title=?2,kind=?3,renewal_date=?4,reminder_days=?5,amount_minor=?6,currency=?7,status=?8,note=?9,updated_at=?10,revision=revision+1 WHERE id=?1",
            params![
                key,
                get("title").as_str(),
                get("kind").as_str(),
                get("renewal_date").as_str(),
                get("reminder_days").as_i64().unwrap_or(14),
                get("amount_minor").as_i64(),
                get("currency").as_str(),
                get("status").as_str(),
                get("note").as_str(),
                now()
            ],
        )?;
        self.record(actor, "administration.update", Some(&key), p)?;
        self.rows("SELECT * FROM administration_items WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }

    pub fn administration_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let n = self.conn.execute(
            "UPDATE administration_items SET deleted_at=?2,revision=revision+1 WHERE id=?1 AND deleted_at IS NULL",
            params![key, now()],
        )?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "administration.delete", Some(&key), p)?;
        Ok(json!({"archived":true,"id":key}))
    }

    /// The personal life-area overview, including the new trackers.
    pub fn personal_overview(&self) -> Result<Value> {
        let interviews = self.interviews_list(&json!({}))?;
        let savings = self.savings_goals_list()?;
        let certifications = self.certifications_list()?;
        let rest = self.rest_intentions_list()?;
        let administration = self.administration_list()?;
        let career = self.domain_list("career", &json!({}))?;
        let learning = self.domain_list("learning", &json!({}))?;
        let finance = self.domain_list("finance", &json!({}))?;
        let wellbeing = self.wellbeing_list(&json!({"limit":30}))?;
        let upcoming_interviews = interviews
            .as_array()
            .map(|rows| {
                rows.iter()
                    .filter(|row| row["status"].as_str() != Some("done"))
                    .take(5)
                    .cloned()
                    .collect::<Vec<Value>>()
            })
            .unwrap_or_default();
        Ok(json!({
            "career": career,
            "learning": learning,
            "finance": finance,
            "wellbeing": wellbeing,
            "interviews": interviews,
            "upcoming_interviews": upcoming_interviews,
            "savings_goals": savings,
            "certifications": certifications,
            "rest_intentions": rest,
            "administration": administration,
            "source":"Local SQLite: career, learning, finance, wellbeing, interview, savings, certification, rest and administration records."
        }))
    }
}
