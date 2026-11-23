use super::*;
fn store() -> Store {
    Store::open(std::env::temp_dir().join(format!("life-project-advanced-{}.sqlite", id())))
        .unwrap()
}

#[test]
fn health_reports_blocked_tasks_and_milestone_overdue() {
    let mut store = store();
    let project = store
        .call("projects.create", &json!({"name":"Planning"}))
        .unwrap();
    let pid = &project["id"];
    let predecessor = store
        .call("tasks.create", &json!({"title":"First","project_id":pid}))
        .unwrap();
    let dependent = store
        .call(
            "tasks.create",
            &json!({"title":"Second","project_id":pid,"due_date":"2020-01-01"}),
        )
        .unwrap();
    store
        .call(
            "dependencies.add",
            &json!({"task_id":predecessor["id"],"blocks_id":dependent["id"]}),
        )
        .unwrap();
    store
        .call(
            "milestones.create",
            &json!({"title":"Release","project_id":pid,"due_date":"2020-01-01"}),
        )
        .unwrap();
    let health = store.call("projects.health", &json!({"id":pid})).unwrap();
    assert_eq!(health["health"], "needs_attention");
    assert_eq!(health["tasks"]["open"], 2);
    assert_eq!(health["tasks"]["blocked"], 1);
    assert_eq!(health["tasks"]["overdue"], 1);
    assert_eq!(health["milestones"]["overdue"], 1);
    assert!(health["methodology"].as_str().unwrap().contains("SQLite"));
    store
        .call("tasks.complete", &json!({"id":predecessor["id"]}))
        .unwrap();
    let health = store.call("projects.health", &json!({"id":pid})).unwrap();
    assert_eq!(health["tasks"]["blocked"], 0);
}
#[test]
fn templates_preserve_tree_edges_and_milestones_without_carrying_due_dates() {
    let mut store = store();
    let source = store
        .call(
            "projects.create",
            &json!({"name":"Shipping","description":"Ship a release"}),
        )
        .unwrap();
    let p = &source["id"];
    let parent = store
        .call(
            "tasks.create",
            &json!({"title":"Plan","project_id":p,"due_date":"2026-11-10"}),
        )
        .unwrap();
    let child = store
        .call(
            "tasks.create",
            &json!({"title":"Implement","project_id":p,"parent_id":parent["id"],"priority":2}),
        )
        .unwrap();
    store
        .call(
            "dependencies.add",
            &json!({"task_id":parent["id"],"blocks_id":child["id"]}),
        )
        .unwrap();
    store
        .call(
            "milestones.create",
            &json!({"title":"Beta","project_id":p,"due_date":"2026-11-15"}),
        )
        .unwrap();
    let template = store
        .call(
            "projects.template.capture",
            &json!({"project_id":p,"name":"Release starter"}),
        )
        .unwrap();
    assert_eq!(template["name"], "Release starter");
    assert_eq!(
        store
            .call("projects.templates.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let new = store
        .call(
            "projects.template.apply",
            &json!({"id":template["id"],"name":"Another release"}),
        )
        .unwrap();
    assert_eq!(new["tasks"].as_array().unwrap().len(), 2);
    assert_eq!(new["milestones"].as_array().unwrap().len(), 1);
    let pid = new["project"]["id"].clone();
    let timeline = store.call("projects.timeline", &json!({"id":pid})).unwrap();
    let new_tasks = timeline["tasks"].as_array().unwrap();
    assert!(new_tasks.iter().all(|t| t["due_date"].is_null()));
    let root = new_tasks.iter().find(|t| t["title"] == "Plan").unwrap();
    let sub = new_tasks
        .iter()
        .find(|t| t["title"] == "Implement")
        .unwrap();
    assert_eq!(sub["parent_id"], root["id"]);
    let edges = store
        .call("dependencies.list", &json!({"task_id":root["id"]}))
        .unwrap();
    assert_eq!(edges.as_array().unwrap().len(), 1);
    assert_eq!(edges[0]["blocks_id"], sub["id"]);
    store
        .call("projects.template.archive", &json!({"id":template["id"]}))
        .unwrap();
    assert_eq!(
        store
            .call("projects.templates.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert!(
        store
            .call("projects.template.apply", &json!({"id":template["id"]}))
            .is_err()
    );
}
#[test]
fn failed_template_apply_is_rolled_back() {
    let mut store = store();
    let before = store
        .call("projects.list", &json!({}))
        .unwrap()
        .as_array()
        .unwrap()
        .len();
    assert!(
        store
            .call("projects.template.apply", &json!({"id":"missing"}))
            .is_err()
    );
    let after = store
        .call("projects.list", &json!({}))
        .unwrap()
        .as_array()
        .unwrap()
        .len();
    assert_eq!(before, after);
}
#[test]
fn migration_to_v12_reopens_without_duplicate_schema() {
    let path = std::env::temp_dir().join(format!("life-project-migration-{}.sqlite", id()));
    {
        let store = Store::open(&path).unwrap();
        let version: i64 = store
            .conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(version, crate::SCHEMA_VERSION);
    }
    let mut store = Store::open(&path).unwrap();
    store
        .call("projects.create", &json!({"name":"Second pass"}))
        .unwrap();
    assert_eq!(
        store
            .call("projects.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
