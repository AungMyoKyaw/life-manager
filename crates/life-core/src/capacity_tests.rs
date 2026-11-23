use super::*;
#[test]
fn deterministic_schedule_respects_protected_intervals_and_events() {
    let mut store =
        Store::open(std::env::temp_dir().join(format!("life-capacity-{}.sqlite", id()))).unwrap();
    store
        .call(
            "availability.replace",
            &json!({"rules":[
                {"weekday":1,"start_time":"09:00","end_time":"13:00","kind":"available"},
                {"weekday":1,"start_time":"10:00","end_time":"10:30","kind":"protected"},
                {"weekday":1,"start_time":"12:00","end_time":"12:30","kind":"break"}
            ]}),
        )
        .unwrap();
    store.call("events.create",&json!({"title":"Meeting","start_at":"2026-11-10T09:00:00","end_at":"2026-11-10T09:45:00","locked":true})).unwrap();
    for i in 0..4 {
        store
            .call(
                "tasks.create",
                &json!({"title":format!("Task {i}"),"estimate_minutes":30}),
            )
            .unwrap();
    }
    let proposal = store
        .call(
            "schedule.propose",
            &json!({"date":"2026-11-10","buffer_minutes":5,"now":"2026-11-10T06:00:00"}),
        )
        .unwrap();
    let slots = proposal["slots"].as_array().unwrap();
    assert!(!slots.is_empty(), "{proposal:?}");
    for block in slots {
        let start = block["start_at"].as_str().unwrap();
        let end = block["end_at"].as_str().unwrap();
        assert!(start >= "2026-11-10T09:45:00" && end <= "2026-11-10T13:00:00");
        assert!(!(start < "2026-11-10T10:30:00" && end > "2026-11-10T10:00:00"));
        assert!(!(start < "2026-11-10T12:30:00" && end > "2026-11-10T12:00:00"));
    }
    assert!(
        proposal["scheduled_minutes"].as_i64().unwrap()
            <= proposal["capacity_minutes"].as_i64().unwrap()
    );
}
#[test]
fn availability_rules_disable_unconfigured_weekends() {
    let mut s =
        Store::open(std::env::temp_dir().join(format!("life-capacity-{}.sqlite", id()))).unwrap();
    s.call(
        "availability.replace",
        &json!({"rules":[{"weekday":0,"start_time":"09:00","end_time":"17:00"}]}),
    )
    .unwrap();
    s.call("tasks.create", &json!({"title":"Weekend project"}))
        .unwrap();
    let plan = s
        .call("schedule.propose", &json!({"date":"2026-11-14"}))
        .unwrap();
    assert_eq!(plan["slots"].as_array().unwrap().len(), 0);
    assert_eq!(plan["unscheduled"].as_array().unwrap().len(), 1);
}
#[test]
fn capacity_does_not_double_book_a_task_with_calendar_block() {
    let mut s =
        Store::open(std::env::temp_dir().join(format!("life-capacity-{}.sqlite", id()))).unwrap();
    let t = s
        .call("tasks.create", &json!({"title":"Already scheduled"}))
        .unwrap();
    s.call("events.create",&json!({"title":"Focus block","task_id":t["id"],"start_at":"2026-11-10T09:00:00","end_at":"2026-11-10T09:30:00"})).unwrap();
    let plan = s
        .call("schedule.propose", &json!({"date":"2026-11-10"}))
        .unwrap();
    assert!(
        plan["slots"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["task_id"] != t["id"])
    );
}
