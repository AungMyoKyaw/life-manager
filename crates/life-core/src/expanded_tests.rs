use super::*;
fn store() -> Store {
    Store::open(std::env::temp_dir().join(format!("life-expanded-{}.sqlite", id()))).unwrap()
}
#[test]
fn recurring_task_generates_next_once() {
    let mut s = store();
    let t = s
        .call(
            "tasks.create",
            &json!({"title":"Daily reading","due_date":"2026-11-09","recurrence":"daily"}),
        )
        .unwrap();
    let completed = s.call("tasks.complete", &json!({"id":t["id"]})).unwrap();
    assert!(completed["next_occurrence_id"].is_string());
    let tasks = s.call("tasks.list", &json!({})).unwrap();
    assert_eq!(tasks.as_array().unwrap().len(), 2);
    assert_eq!(tasks[0]["due_date"], "2026-11-10");
    s.call("tasks.complete", &json!({"id":t["id"]})).unwrap();
    assert_eq!(
        s.call("tasks.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
#[test]
fn idempotency_rejects_payload_reuse() {
    let mut s = store();
    let p = json!({"request_id":"once","title":"One"});
    s.call("tasks.create", &p).unwrap();
    assert!(
        s.call(
            "tasks.create",
            &json!({"request_id":"once","title":"Changed"})
        )
        .is_err()
    );
    assert_eq!(
        s.call("tasks.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn routines_reviews_and_milestones() {
    let mut s = store();
    let routine = s
        .call("routines.create", &json!({"title":"Morning"}))
        .unwrap();
    s.call(
        "routines.step.add",
        &json!({"routine_id":routine["id"],"title":"Meditate","position":0}),
    )
    .unwrap();
    assert_eq!(
        s.call("routines.steps", &json!({"routine_id":routine["id"]}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let review=s.call("reviews.create",&json!({"title":"Weekly review","period":"weekly","day":"2026-11-09","content":"Learned a lot"})).unwrap();
    assert!(review["id"].is_string());
    let project = s
        .call("projects.create", &json!({"name":"Project Test"}))
        .unwrap();
    let milestone = s
        .call(
            "milestones.create",
            &json!({"project_id":project["id"],"title":"Ship release"}),
        )
        .unwrap();
    s.call("milestones.complete", &json!({"id":milestone["id"]}))
        .unwrap();
    assert_eq!(
        s.call("milestones.list", &json!({"project_id":project["id"]}))
            .unwrap()[0]["status"],
        "done"
    );
}
#[test]
fn focus_pause_resume_and_stop() {
    let mut s = store();
    let start = s.call("focus.start", &json!({})).unwrap();
    s.call("focus.pause", &json!({"id":start["id"]})).unwrap();
    assert!(s.call("focus.start", &json!({})).is_err());
    s.call("focus.resume", &json!({"id":start["id"]})).unwrap();
    let done = s.call("focus.stop", &json!({"id":start["id"]})).unwrap();
    assert_eq!(done["status"], "completed");
}
#[test]
fn dependency_cycles_rejected() {
    let mut s = store();
    let a = s.call("tasks.create", &json!({"title":"A"})).unwrap();
    let b = s.call("tasks.create", &json!({"title":"B"})).unwrap();
    s.call(
        "dependencies.add",
        &json!({"task_id":a["id"],"blocks_id":b["id"]}),
    )
    .unwrap();
    assert!(
        s.call(
            "dependencies.add",
            &json!({"task_id":b["id"],"blocks_id":a["id"]})
        )
        .is_err()
    );
}
#[test]
fn automation_manual_run_is_audited() {
    let mut s = store();
    let rule=s.call("automations.create",&json!({"title":"Make task","action":"tasks.create","arguments":{"title":"Automated task"}})).unwrap();
    let result = s
        .call("automations.run", &json!({"id":rule["id"]}))
        .unwrap();
    assert_eq!(result["result"]["title"], "Automated task");
    assert_eq!(
        s.call("automations.history", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn migration_survives_reopen() {
    let path = std::env::temp_dir().join(format!("life-reopen-{}.sqlite", id()));
    {
        let mut s = Store::open(&path).unwrap();
        s.call("tasks.create", &json!({"title":"Before restart"}))
            .unwrap();
    }
    let mut reopened = Store::open(&path).unwrap();
    assert_eq!(
        reopened
            .call("tasks.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        reopened
            .conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        crate::SCHEMA_VERSION
    );
}

#[test]
fn schedule_apply_is_atomic_and_conflict_aware() {
    let mut s = store();
    let t = s
        .call("tasks.create", &json!({"title":"Task to schedule"}))
        .unwrap();
    let task_id = t["id"].clone();
    let first =
        json!({"task_id":task_id,"start_at":"2026-11-16T09:00:00","end_at":"2026-11-16T10:00:00"});
    let invalid = json!({"task_id":"missing-task","start_at":"2026-11-16T11:00:00","end_at":"2026-11-16T12:00:00"});
    assert!(
        s.call("schedule.apply", &json!({"slots":[first.clone(),invalid]}))
            .is_err()
    );
    assert_eq!(
        s.call("events.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
    let applied = s
        .call("schedule.apply", &json!({"slots":[first.clone()]}))
        .unwrap();
    assert_eq!(applied["count"], 1);
    assert!(s.call("schedule.apply", &json!({"slots":[first]})).is_err());
    assert_eq!(
        s.call("events.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn idempotent_schedule_replay_avoids_duplicates() {
    let mut s = store();
    let task = s
        .call("tasks.create", &json!({"title":"Agent focus"}))
        .unwrap();
    let p = json!({"request_id":"agent-unique-slot","slots":[{"task_id":task["id"],"start_at":"2026-11-18T09:00:00","end_at":"2026-11-18T10:00:00"}]});
    let first = s.call("schedule.apply", &p).unwrap();
    let second = s.call("schedule.apply", &p).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        s.call("events.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn area_lifecycle_and_deleted_task_restore() {
    let mut s = store();
    let area = s
        .call("areas.create", &json!({"name":"Family","color":"#267755"}))
        .unwrap();
    let aid = area["id"].as_str().unwrap().to_string();
    assert_eq!(
        s.call("areas.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["id"] == aid)
            .count(),
        1
    );
    let updated = s
        .call(
            "areas.update",
            &json!({"id":aid,"name":"Family and friends"}),
        )
        .unwrap();
    assert_eq!(updated["name"], "Family and friends");
    let task = s
        .call(
            "tasks.create",
            &json!({"title":"Write a letter","area_id":aid}),
        )
        .unwrap();
    s.call("tasks.delete", &json!({"id":task["id"]})).unwrap();
    assert!(
        s.call("tasks.list", &json!({"query":"Write a letter"}))
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    let restored = s.call("tasks.restore", &json!({"id":task["id"]})).unwrap();
    assert_eq!(restored["title"], "Write a letter");
    assert_eq!(restored["revision"], 3);
    s.call("areas.delete", &json!({"id":aid})).unwrap();
    assert!(
        s.call("areas.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["id"] != aid)
    );
    assert_eq!(
        s.call("tasks.get", &json!({"id":task["id"]})).unwrap()["area_id"],
        aid
    );
}
#[test]
fn verified_restore_preserves_previous_database() {
    let mut s = store();
    s.call("tasks.create", &json!({"title":"Before snapshot"}))
        .unwrap();
    let backup = s.call("backup.create", &json!({})).unwrap();
    assert_eq!(
        s.call("backup.verify", &json!({"path":backup["path"]}))
            .unwrap()["verified"],
        true
    );
    s.call("tasks.create", &json!({"title":"After snapshot"}))
        .unwrap();
    assert!(
        s.call("backup.restore", &json!({"path":backup["path"]}))
            .is_err()
    );
    let restored = s
        .call(
            "backup.restore",
            &json!({"path":backup["path"],"confirm":true}),
        )
        .unwrap();
    assert_eq!(restored["restored"], true);
    let titles: Vec<String> = s
        .call("tasks.list", &json!({}))
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["title"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(titles, vec!["Before snapshot"]);
    assert!(
        restored["pre_restore_backup"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );
}
