//! Executable saved views, saved searches and bidirectional backlinks.
use super::*;

const FILTER_KEYS: [&str; 11] = [
    "status",
    "project_id",
    "area_id",
    "goal_id",
    "priority",
    "query",
    "due_from",
    "due_to",
    "tags",
    "kind",
    "limit",
];

impl Store {
    /// Run a saved filter and return the matching rows.
    pub fn saved_views_run(&self, p: &Value) -> Result<Value> {
        let key = require(p, "id")?;
        let row = self
            .rows("SELECT * FROM saved_filters WHERE id=?1", &[&key])?
            .as_array()
            .and_then(|v| v.first())
            .cloned()
            .ok_or_else(|| LifeError::NotFound(key.clone()))?;
        let entity = row["entity"].as_str().unwrap_or("tasks").to_string();
        let query: Value = serde_json::from_str(row["query_json"].as_str().unwrap_or("{}"))
            .map_err(|_| LifeError::Invalid("Stored filter is not valid JSON".into()))?;
        let rows = self.run_filter(&entity, &query)?;
        let count = rows.as_array().map_or(0, Vec::len);
        Ok(
            json!({"id":key,"entity":entity,"name":row["name"].as_str().unwrap_or(""),"query":query,"results":rows,"count":count}),
        )
    }

    /// Evaluate a filter object against an entity table.
    pub fn run_filter(&self, entity: &str, query: &Value) -> Result<Value> {
        if query.as_object().is_none() {
            return Err(LifeError::Invalid("Filter must be an object".into()));
        }
        if let Some(object) = query.as_object()
            && object.keys().any(|k| !FILTER_KEYS.contains(&k.as_str()))
        {
            return Err(LifeError::Invalid("Unknown saved-filter property".into()));
        }
        let get = |k: &str| {
            query
                .get(k)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        let limit = query
            .get("limit")
            .and_then(Value::as_i64)
            .unwrap_or(250)
            .clamp(1, 1000);
        let status = get("status");
        let project = get("project_id");
        let area = get("area_id");
        let goal = get("goal_id");
        let priority = query.get("priority").and_then(Value::as_i64).unwrap_or(0);
        let text_query = get("query");
        let due_from = get("due_from");
        let due_to = get("due_to");
        let kind = query
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        match entity {
            "tasks" => self.rows(
                "SELECT t.*,p.name AS project_name,a.name AS area_name FROM tasks t LEFT JOIN projects p ON t.project_id=p.id LEFT JOIN areas a ON t.area_id=a.id WHERE t.deleted_at IS NULL AND (?1='' OR t.status=?1) AND (?2='' OR t.project_id=?2) AND (?3='' OR t.area_id=?3) AND (?4='' OR t.goal_id=?4) AND (?5=0 OR t.priority=?5) AND (?6='' OR t.title LIKE '%'||?6||'%' OR t.description LIKE '%'||?6||'%') AND (?7='' OR IFNULL(t.due_date,'')>=?7) AND (?8='' OR IFNULL(t.due_date,'')<=?8) ORDER BY t.priority ASC, COALESCE(t.due_date,'9999-12-31') ASC LIMIT ?9",
                &[&status, &project, &area, &goal, &priority, &text_query, &due_from, &due_to, &limit],
            ),
            "projects" => self.rows(
                "SELECT * FROM projects WHERE (?1='' OR status=?1) AND (?2='' OR area_id=?2) AND (?3='' OR name LIKE '%'||?3||'%' OR description LIKE '%'||?3||'%') ORDER BY updated_at DESC LIMIT ?4",
                &[&status, &area, &text_query, &limit],
            ),
            "notes" => self.rows(
                "SELECT id,title,substr(content,1,400) AS content,area_id,updated_at FROM notes WHERE (?1='' OR title LIKE '%'||?1||'%' OR content LIKE '%'||?1||'%') AND (?2='' OR area_id=?2) ORDER BY updated_at DESC LIMIT ?3",
                &[&text_query, &area, &limit],
            ),
            "events" => self.rows(
                "SELECT * FROM events WHERE (?1='' OR kind=?1) AND (?2='' OR title LIKE '%'||?2||'%') AND (?3='' OR IFNULL(start_at,'')>=?3) AND (?4='' OR IFNULL(start_at,'')<=?4) ORDER BY start_at LIMIT ?5",
                &[&kind, &text_query, &due_from, &due_to, &limit],
            ),
            _ => Err(LifeError::Invalid("Unsupported filter entity".into())),
        }
    }

    pub fn saved_searches_list(&self) -> Result<Value> {
        self.rows("SELECT * FROM saved_searches ORDER BY created_at DESC", &[])
    }

    pub fn saved_searches_create(&self, p: &Value, actor: &str) -> Result<Value> {
        let name = require(p, "name")?;
        if name.chars().count() > 180 {
            return Err(LifeError::Invalid("name exceeds 180 characters".into()));
        }
        let entity = optional(p, "entity").unwrap_or_else(|| "all".into());
        if !["all", "tasks", "notes", "projects", "events", "goals"].contains(&entity.as_str()) {
            return Err(LifeError::Invalid("Unsupported search entity".into()));
        }
        let query = p
            .get("query")
            .ok_or_else(|| LifeError::Invalid("query required".into()))?;
        if !query.is_object() {
            return Err(LifeError::Invalid("query must be an object".into()));
        }
        if let Some(object) = query.as_object()
            && object.keys().any(|k| !FILTER_KEYS.contains(&k.as_str()))
        {
            return Err(LifeError::Invalid("Unknown saved-search property".into()));
        }
        let key = id();
        self.conn.execute(
            "INSERT INTO saved_searches(id,name,entity,query_json,created_at) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(name) DO UPDATE SET entity=excluded.entity,query_json=excluded.query_json",
            params![key, name, entity, query.to_string(), now()],
        )?;
        self.record(actor, "saved_searches.create", Some(&key), p)?;
        self.rows("SELECT * FROM saved_searches WHERE id=?1", &[&key])
            .map(|v| v[0].clone())
    }

    pub fn saved_searches_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let n = self
            .conn
            .execute("DELETE FROM saved_searches WHERE id=?1", [&key])?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "saved_searches.delete", Some(&key), p)?;
        Ok(json!({"deleted":true,"id":key}))
    }

    /// Run a saved search across one entity or across everything.
    pub fn saved_searches_run(&self, p: &Value) -> Result<Value> {
        let key = require(p, "id")?;
        let row = self
            .rows("SELECT * FROM saved_searches WHERE id=?1", &[&key])?
            .as_array()
            .and_then(|v| v.first())
            .cloned()
            .ok_or_else(|| LifeError::NotFound(key.clone()))?;
        let entity = row["entity"].as_str().unwrap_or("all").to_string();
        let query: Value = serde_json::from_str(row["query_json"].as_str().unwrap_or("{}"))
            .map_err(|_| LifeError::Invalid("Stored search is not valid JSON".into()))?;
        let mut out = serde_json::Map::new();
        let mut total = 0usize;
        let entities: Vec<&str> = if entity == "all" {
            vec!["tasks", "projects", "notes", "events", "goals"]
        } else {
            vec![entity.as_str()]
        };
        for name in entities {
            if name == "goals" {
                let rows = self.rows(
                    "SELECT * FROM goals WHERE (?1='' OR title LIKE '%'||?1||'%') AND (?2='' OR area_id=?2) AND (?3='' OR status=?3) LIMIT ?4",
                    &[
                        &query.get("query").and_then(Value::as_str).unwrap_or(""),
                        &query.get("area_id").and_then(Value::as_str).unwrap_or(""),
                        &query.get("status").and_then(Value::as_str).unwrap_or(""),
                        &limit_value(&query),
                    ],
                )?;
                total += rows.as_array().map_or(0, Vec::len);
                out.insert("goals".into(), rows);
                continue;
            }
            let rows = self.run_filter(name, &query)?;
            total += rows.as_array().map_or(0, Vec::len);
            out.insert(name.to_string(), rows);
        }
        Ok(
            json!({"id":key,"name":row["name"].as_str().unwrap_or(""),"entity":entity,"query":query,"results":Value::Object(out),"count":total}),
        )
    }

    /// Everything linked to an entity, in both directions.
    pub fn backlinks(&self, p: &Value) -> Result<Value> {
        let kind = require(p, "kind")?;
        let key = require(p, "id")?;
        let rows = self.rows(
            "SELECT l.id,l.source_kind,l.source_id,l.target_kind,l.target_id,l.relation,l.created_at FROM entity_links l WHERE (l.source_kind=?1 AND l.source_id=?2) OR (l.target_kind=?1 AND l.target_id=?2) ORDER BY l.created_at DESC",
            &[&kind, &key],
        )?;
        let mut out: Vec<Value> = Vec::new();
        for link in rows.as_array().into_iter().flatten() {
            let outgoing = link["source_kind"].as_str() == Some(kind.as_str())
                && link["source_id"].as_str() == Some(key.as_str());
            let (other_kind, other_id) = if outgoing {
                (link["target_kind"].clone(), link["target_id"].clone())
            } else {
                (link["source_kind"].clone(), link["source_id"].clone())
            };
            out.push(json!({
                "link_id":link["id"].clone(),
                "direction": if outgoing { "outgoing" } else { "incoming" },
                "relation":link["relation"].clone(),
                "kind":other_kind,
                "id":other_id,
                "title":self.link_title(&other_kind, &other_id),
            }));
        }
        let count = out.len();
        Ok(json!({"kind":kind,"id":key,"links":out,"count":count}))
    }

    fn link_title(&self, kind: &Value, key: &Value) -> Value {
        let table = match kind.as_str().unwrap_or("") {
            "notes" => "notes",
            "tasks" => "tasks",
            "projects" => "projects",
            "goals" => "goals",
            _ => "",
        };
        if table.is_empty() {
            return Value::Null;
        }
        let column = if table == "projects" { "name" } else { "title" };
        self.rows(
            &format!("SELECT {column} AS title FROM {table} WHERE id=?1"),
            &[&key.as_str().unwrap_or("")],
        )
        .ok()
        .and_then(|v| {
            v.as_array()
                .and_then(|a| a.first())
                .map(|r| r["title"].clone())
        })
        .unwrap_or(Value::Null)
    }
}

fn limit_value(query: &Value) -> i64 {
    query
        .get("limit")
        .and_then(Value::as_i64)
        .unwrap_or(250)
        .clamp(1, 1000)
}
