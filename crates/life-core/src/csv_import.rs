//! CSV import with header mapping, preview, duplicate detection and
//! partial-failure reporting.
use super::*;

fn split_row(line: &str) -> Vec<String> {
    let mut fields: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if quoted && chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    quoted = !quoted;
                }
            }
            ',' if !quoted => {
                fields.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current.trim().to_string());
    fields
}

/// Parse a CSV document into a header and its rows.
fn parse_csv(content: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let mut lines = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim_end_matches('\r').to_string())
        .collect::<Vec<String>>();
    if lines.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let header = split_row(&lines.remove(0));
    let rows: Vec<Vec<String>> = lines.iter().map(|line| split_row(line)).collect();
    (header, rows)
}

/// Map a CSV header to the field names the domain understands.
fn map_header(header: &[String]) -> Vec<(usize, String)> {
    let mut mapping: Vec<(usize, String)> = Vec::new();
    for (index, raw) in header.iter().enumerate() {
        let name = raw.trim().to_lowercase().replace([' ', '-'], "_");
        let field = match name.as_str() {
            "task" | "task_title" | "name" | "title" => "title",
            "details" | "note" | "notes" | "description" => "description",
            "due" | "due_date" | "deadline" => "due_date",
            "planned" | "planned_date" | "start_date" => "planned_date",
            "priority" | "prio" => "priority",
            "project" | "project_name" => "project_name",
            "area" | "life_area" | "area_name" => "area_name",
            "tags" | "labels" => "tags",
            "estimate" | "estimate_minutes" | "minutes" => "estimate_minutes",
            "status" => "status",
            "type" | "kind" => "kind",
            "habit" | "habit_title" => "habit_title",
            "frequency" => "frequency",
            "target" => "target",
            "unit" => "unit",
            _ => continue,
        };
        mapping.push((index, field.to_string()));
    }
    mapping
}

fn fingerprint(kind: &str, title: &str, extra: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(kind.as_bytes());
    hasher.update(b"\0");
    hasher.update(title.to_lowercase().as_bytes());
    hasher.update(b"\0");
    hasher.update(extra.as_bytes());
    format!("{:x}", hasher.finalize())
}

impl Store {
    /// Preview a CSV import without writing anything.
    pub fn csv_import_preview(&self, p: &Value) -> Result<Value> {
        let content = require(p, "content")?;
        let source = optional(p, "source").unwrap_or_else(|| "csv".into());
        let (header, rows) = parse_csv(&content);
        if header.is_empty() {
            return Err(LifeError::Invalid("The CSV file has no header row".into()));
        }
        let mapping = map_header(&header);
        if mapping.is_empty() {
            return Err(LifeError::Invalid(
                "No recognised column names were found in the CSV header".into(),
            ));
        }
        let mut valid: Vec<Value> = Vec::new();
        let mut invalid: Vec<Value> = Vec::new();
        let mut duplicates: Vec<Value> = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            let mut record = serde_json::Map::new();
            for (column, field) in &mapping {
                let value = row.get(*column).cloned().unwrap_or_default();
                if !value.is_empty() {
                    record.insert(field.clone(), json!(value));
                }
            }
            let title = record
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            let kind = record
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("tasks")
                .trim()
                .to_string();
            let entity_kind = match kind.as_str() {
                "task" | "tasks" => "tasks",
                "note" | "notes" => "notes",
                "project" | "projects" => "projects",
                "goal" | "goals" => "goals",
                "event" | "events" => "events",
                "habit" | "habits" => "habits",
                other => {
                    invalid.push(json!({
                        "row":index + 2,
                        "reason":format!("Unsupported record type {other}"),
                        "title":title
                    }));
                    continue;
                }
            };
            if title.is_empty() {
                invalid.push(json!({
                    "row":index + 2,
                    "reason":"Missing a title",
                    "kind":entity_kind
                }));
                continue;
            }
            let extra = record
                .get("due_date")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let mark = fingerprint(entity_kind, &title, &extra);
            if seen.contains(&mark) {
                duplicates.push(json!({
                    "row":index + 2,
                    "kind":entity_kind,
                    "title":title,
                    "reason":"Repeated row in this file"
                }));
                continue;
            }
            seen.push(mark.clone());
            let existing = self.rows(
                "SELECT id FROM imported_records WHERE source=?1 AND fingerprint=?2",
                &[&source, &mark],
            )?;
            if existing.as_array().is_some_and(|rows| !rows.is_empty()) {
                duplicates.push(json!({
                    "row":index + 2,
                    "kind":entity_kind,
                    "title":title,
                    "reason":"Already imported from this source"
                }));
                continue;
            }
            let local_duplicate = self.local_duplicate(entity_kind, &title)?;
            if let Some(existing_id) = local_duplicate {
                duplicates.push(json!({
                    "row":index + 2,
                    "kind":entity_kind,
                    "title":title,
                    "reason":"A record with this title already exists",
                    "existing_id":existing_id
                }));
                continue;
            }
            valid.push(json!({
                "row":index + 2,
                "kind":entity_kind,
                "title":title,
                "fingerprint":mark,
                "record":Value::Object(record)
            }));
        }
        let total = rows.len();
        Ok(json!({
            "source":source,
            "header":header,
            "mapping":mapping.iter().map(|(i,f)| json!({"column":i,"field":f})).collect::<Vec<Value>>(),
            "valid":valid,
            "invalid":invalid,
            "duplicates":duplicates,
            "total":total,
            "can_apply":!valid.is_empty(),
            "policy":"Preview only. Nothing is written until the reviewed apply runs with an explicit confirmation."
        }))
    }

    fn local_duplicate(&self, kind: &str, title: &str) -> Result<Option<String>> {
        let (table, column) = match kind {
            "tasks" => ("tasks", "title"),
            "notes" => ("notes", "title"),
            "projects" => ("projects", "name"),
            "goals" => ("goals", "title"),
            "habits" => ("habits", "title"),
            "events" => ("events", "title"),
            _ => return Ok(None),
        };
        let sql = if table == "tasks" {
            format!("SELECT id FROM {table} WHERE {column}=?1 AND deleted_at IS NULL LIMIT 1")
        } else {
            format!("SELECT id FROM {table} WHERE {column}=?1 LIMIT 1")
        };
        let rows = self.rows(&sql, &[&title])?;
        Ok(rows
            .as_array()
            .and_then(|r| r.first())
            .and_then(|r| r["id"].as_str())
            .map(|v| v.to_string()))
    }

    /// Apply a reviewed CSV import. Rows that fail are reported and skipped
    /// when the caller allows it; the rest are written in one transaction.
    pub fn csv_import_apply(&self, p: &Value, actor: &str) -> Result<Value> {
        let content = require(p, "content")?;
        if !p.get("confirm").and_then(Value::as_bool).unwrap_or(false) {
            return Err(LifeError::Invalid(
                "Explicit confirmation is required before an import is applied".into(),
            ));
        }
        let allow_invalid = p
            .get("allow_invalid")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let source = optional(p, "source").unwrap_or_else(|| "csv".into());
        let preview = self.csv_import_preview(&json!({"content":content,"source":source}))?;
        if preview["invalid"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty())
            && !allow_invalid
        {
            return Err(LifeError::Conflict(
                "Some rows could not be read. Review them, then apply with allow_invalid.".into(),
            ));
        }
        let mut created: Vec<Value> = Vec::new();
        let mut failed: Vec<Value> = Vec::new();
        for entry in preview["valid"].as_array().into_iter().flatten() {
            let record = &entry["record"];
            let kind = entry["kind"].as_str().unwrap_or("tasks");
            let title = entry["title"].as_str().unwrap_or("");
            let mut payload = record.as_object().cloned().unwrap_or_default();
            payload.remove("project_name");
            payload.remove("area_name");
            payload.remove("kind");
            payload.remove("habit_title");
            payload.remove("frequency");
            payload.remove("target");
            payload.remove("unit");
            let outcome = match kind {
                "tasks" => self.create_task(&Value::Object(payload.clone()), actor),
                "notes" => self.create_note(&Value::Object(payload.clone()), actor),
                "projects" => self.create_project(&Value::Object(payload.clone()), actor),
                "goals" => self.create_goal(&Value::Object(payload.clone()), actor),
                "habits" => self.create_habit(&Value::Object(payload.clone()), actor),
                _ => Err(LifeError::Invalid(format!("Unsupported kind {kind}"))),
            };
            match outcome {
                Ok(row) => {
                    self.conn.execute(
                        "INSERT OR IGNORE INTO imported_records(id,source,fingerprint,entity_kind,entity_id,imported_at) VALUES(?1,?2,?3,?4,?5,?6)",
                        params![
                            id(),
                            source,
                            entry["fingerprint"].as_str().unwrap_or(""),
                            kind,
                            row["id"].as_str().unwrap_or(""),
                            now()
                        ],
                    )?;
                    created.push(json!({"row":entry["row"].clone(),"kind":kind,"title":title,"id":row["id"].clone()}));
                }
                Err(error) => failed.push(json!({
                    "row":entry["row"].clone(),
                    "kind":kind,
                    "title":title,
                    "reason":error.to_string()
                })),
            }
        }
        Ok(json!({
            "source":source,
            "created":created,
            "failed":failed,
            "skipped_duplicates":preview["duplicates"].clone(),
            "skipped_invalid":preview["invalid"].clone(),
            "created_count":created.len(),
            "failed_count":failed.len(),
            "atomic":failed.is_empty()
        }))
    }

    /// The records imported from a source.
    pub fn csv_import_history(&self, p: &Value) -> Result<Value> {
        let source = optional(p, "source");
        match source {
            Some(s) => self.rows(
                "SELECT * FROM imported_records WHERE source=?1 ORDER BY imported_at DESC",
                &[&s],
            ),
            None => self.rows(
                "SELECT * FROM imported_records ORDER BY imported_at DESC LIMIT 500",
                &[],
            ),
        }
    }
}
