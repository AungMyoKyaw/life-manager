use super::*;
fn store() -> Store {
    Store::open(std::env::temp_dir().join(format!("life-parity-{}.sqlite", id()))).unwrap()
}
#[test]
fn templates_and_atomic_bulk() {
    let mut s = store();
    let template = s
        .call(
            "templates.create",
            &json!({"name":"Daily plan","title":"Review top goals","priority":1}),
        )
        .unwrap();
    let a = s
        .call("templates.apply", &json!({"id":template["id"]}))
        .unwrap();
    let b = s
        .call("tasks.create", &json!({"title":"Write notes"}))
        .unwrap();
    let result = s
        .call(
            "tasks.bulk",
            &json!({"ids":[a["id"],b["id"]],"fields":{"priority":2}}),
        )
        .unwrap();
    assert_eq!(result["count"], 2);
    let original = s.call("tasks.get", &json!({"id":a["id"]})).unwrap();
    assert!(
        s.call(
            "tasks.bulk",
            &json!({"ids":[a["id"],"unknown"],"fields":{"priority":1}})
        )
        .is_err()
    );
    let after = s.call("tasks.get", &json!({"id":a["id"]})).unwrap();
    assert_eq!(after["priority"], original["priority"]);
}
#[test]
fn key_results_update_goal_progress_and_history() {
    let mut s = store();
    let goal = s
        .call(
            "goals.create",
            &json!({"title":"Learn new skills","target":100}),
        )
        .unwrap();
    let kr = s
        .call(
            "key_results.create",
            &json!({"goal_id":goal["id"],"title":"Complete exercises","baseline":0,"target":20}),
        )
        .unwrap();
    let updated = s
        .call("key_results.update", &json!({"id":kr["id"],"current":10}))
        .unwrap();
    assert_eq!(updated["current"], 10.0);
    let goals = s.call("goals.list", &json!({})).unwrap();
    assert_eq!(goals[0]["progress"], 50.0);
    let history = s
        .call("goals.history", &json!({"goal_id":goal["id"]}))
        .unwrap();
    assert_eq!(history.as_array().unwrap().len(), 1);
}
#[test]
fn links_and_availability_are_persistent() {
    let mut s = store();
    let task = s
        .call("tasks.create", &json!({"title":"Write feature"}))
        .unwrap();
    let note = s
        .call("notes.create", &json!({"title":"Architecture"}))
        .unwrap();
    let link=s.call("links.add",&json!({"source_kind":"notes","source_id":note["id"],"target_kind":"tasks","target_id":task["id"]})).unwrap();
    assert_eq!(
        s.call("links.list", &json!({"kind":"tasks","id":task["id"]}))
            .unwrap()[0]["id"],
        link["id"]
    );
    let slots=s.call("availability.replace",&json!({"rules":[{"weekday":0,"start_time":"09:00","end_time":"17:30","kind":"available"}]})).unwrap();
    assert_eq!(slots.as_array().unwrap().len(), 1);
    assert!(
        s.call(
            "availability.replace",
            &json!({"rules":[{"weekday":9,"start_time":"10:00","end_time":"11:00"}]})
        )
        .is_err()
    );
    assert_eq!(
        s.call("availability.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn saved_views_and_import_preview_commit() {
    let mut s = store();
    let f = s
        .call(
            "saved_views.create",
            &json!({"name":"Important","query":{"priority":1}}),
        )
        .unwrap();
    assert_eq!(
        s.call("saved_views.list", &json!({})).unwrap()[0]["id"],
        f["id"]
    );
    let items = json!([{"type":"tasks","title":"New imported task"},{"type":"notes","title":"Imported note"},{"type":"x","title":"Invalid"}]);
    let preview = s.call("import.preview", &json!({"items":items})).unwrap();
    assert_eq!(preview["valid"].as_array().unwrap().len(), 2);
    assert_eq!(preview["invalid"].as_array().unwrap().len(), 1);
    assert!(
        s.call("import.apply", &json!({"items":items,"confirm":true}))
            .is_err()
    );
    let applied = s
        .call(
            "import.apply",
            &json!({"items":items,"confirm":true,"allow_invalid":true}),
        )
        .unwrap();
    assert_eq!(applied["created"].as_array().unwrap().len(), 2);
    assert_eq!(
        s.call("import.preview", &json!({"items":items})).unwrap()["duplicates"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
#[test]
fn report_and_ics_are_portable() {
    let mut s = store();
    let e=s.call("events.create",&json!({"title":"Check-in","start_at":"2026-11-09T09:00:00","end_at":"2026-11-09T10:00:00"})).unwrap();
    assert!(e["id"].is_string());
    let ics = s.call("calendar.export_ics", &json!({})).unwrap();
    assert!(ics["content"].as_str().unwrap().contains("BEGIN:VEVENT"));
    let report = s
        .call(
            "reports.summary",
            &json!({"from":"2026-11-01","to":"2026-11-30"}),
        )
        .unwrap();
    assert!(report["completed_tasks"].is_number());
}
#[test]
fn agent_undo_respects_newer_changes_and_restores_previous_values() {
    let mut s = store();
    let t = s
        .call(
            "tasks.create",
            &json!({"title":"Before","priority":3,"request_id":"create-1"}),
        )
        .unwrap();
    let first = s
        .call(
            "tasks.update",
            &json!({"id":t["id"],"title":"After","priority":1,"request_id":"update-1"}),
        )
        .unwrap();
    assert_eq!(first["title"], "After");
    let changes = s.call("agents.changes", &json!({})).unwrap();
    let update_id = changes
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["action"] == "tasks.update")
        .unwrap()["id"]
        .clone();
    s.call("agents.undo", &json!({"id":update_id})).unwrap();
    assert_eq!(
        s.call("tasks.get", &json!({"id":t["id"]})).unwrap()["title"],
        "Before"
    );
    assert!(s.call("agents.undo", &json!({"id":update_id})).is_err());
    let next = s
        .call("tasks.update", &json!({"id":t["id"],"title":"Second"}))
        .unwrap();
    let earlier = s
        .call("agents.changes", &json!({}))
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["action"] == "tasks.create")
        .unwrap()["id"]
        .clone();
    assert!(s.call("agents.undo", &json!({"id":earlier})).is_err());
    assert_eq!(
        s.call("tasks.get", &json!({"id":t["id"]})).unwrap()["title"],
        next["title"]
    );
}
#[test]
fn agent_undo_recovers_soft_deleted_task() {
    let mut s = store();
    let t = s
        .call("tasks.create", &json!({"title":"Recover me"}))
        .unwrap();
    s.call("tasks.delete", &json!({"id":t["id"]})).unwrap();
    let events = s.call("agents.changes", &json!({})).unwrap();
    let key = events
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["action"] == "tasks.delete")
        .unwrap()["id"]
        .clone();
    let u = s.call("agents.undo", &json!({"id":key})).unwrap();
    assert_eq!(u["undone"], true);
    assert_eq!(
        s.call("tasks.get", &json!({"id":t["id"]})).unwrap()["title"],
        "Recover me"
    );
}
