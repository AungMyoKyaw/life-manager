use super::*;
fn db() -> Store {
    Store::open(std::env::temp_dir().join(format!("life-reminder-test-{}.sqlite", id()))).unwrap()
}
fn enabled(s: &mut Store) {
    s.call("settings.set",&json!({"key":"notification.preferences","value":{"enabled":true,"quiet_start":"22:00","quiet_end":"08:00"}})).unwrap();
}
#[test]
fn reminders_are_opt_in_and_respect_quiet_hours() {
    let mut s = db();
    s.call(
        "tasks.create",
        &json!({"title":"Deadline reminder","due_date":"2026-11-10"}),
    )
    .unwrap();
    let now = "2026-11-10T09:01:00";
    assert_eq!(
        s.call("reminders.due", &json!({"now":now}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
    enabled(&mut s);
    assert_eq!(
        s.call("reminders.due", &json!({"now":now,"lookback_minutes":30}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        s.call(
            "reminders.due",
            &json!({"now":"2026-11-10T23:00:00","lookback_minutes":900})
        )
        .unwrap()
        .as_array()
        .unwrap()
        .len(),
        0
    );
    assert_eq!(
        s.call("reminders.settings", &json!({})).unwrap()["enabled"],
        true
    );
}
#[test]
fn upcoming_reminders_aggregate_personal_domains_and_calendar() {
    let mut s = db();
    enabled(&mut s);
    s.call(
        "tasks.create",
        &json!({"title":"Task due","due_date":"2026-11-10"}),
    )
    .unwrap();
    s.call("events.create",&json!({"title":"Morning review","start_at":"2026-11-10T09:15:00","end_at":"2026-11-10T10:00:00"})).unwrap();
    s.call("career.create",&json!({"employer":"Example","role":"Engineer","next_action_on":"2026-11-10","next_action":"Review follow-up"})).unwrap();
    s.call(
        "learning.create",
        &json!({"title":"Kubernetes","due_date":"2026-11-10"}),
    )
    .unwrap();
    s.call(
        "finance.create",
        &json!({"title":"Bill","due_date":"2026-11-10"}),
    )
    .unwrap();
    let due = s
        .call(
            "reminders.due",
            &json!({"now":"2026-11-10T09:01:00","lookback_minutes":10}),
        )
        .unwrap();
    let ids = due
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v["source_kind"].as_str())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), 5, "{due:?}");
}
#[test]
fn reminder_claim_delivery_dismiss_snooze_are_deduplicated() {
    let mut s = db();
    enabled(&mut s);
    s.call(
        "tasks.create",
        &json!({"title":"Finish review","due_date":"2026-11-10"}),
    )
    .unwrap();
    let now = "2026-11-10T09:02:00";
    let due = s.call("reminders.due", &json!({"now":now})).unwrap();
    let key = due[0]["key"].as_str().unwrap().to_owned();
    let first = s
        .call("reminders.claim", &json!({"key":key,"now":now}))
        .unwrap();
    assert_eq!(first["claimed"], true);
    let second = s
        .call("reminders.claim", &json!({"key":key,"now":now}))
        .unwrap();
    assert_eq!(second["claimed"], false);
    s.call("reminders.delivered", &json!({"key":key,"now":now}))
        .unwrap();
    assert_eq!(
        s.call("reminders.due", &json!({"now":now}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert!(
        s.call("reminders.delivered", &json!({"key":key,"now":now}))
            .is_err()
    );
    s.call(
        "reminders.snooze",
        &json!({"key":key,"now":now,"until":"2026-11-10T10:00:00"}),
    )
    .unwrap();
    assert_eq!(
        s.call("reminders.due", &json!({"now":"2026-11-10T09:03:00"}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        s.call(
            "reminders.due",
            &json!({"now":"2026-11-10T10:00:00","lookback_minutes":60})
        )
        .unwrap()
        .as_array()
        .unwrap()
        .len(),
        1
    );
    s.call("reminders.dismiss", &json!({"key":key})).unwrap();
    assert!(
        s.call(
            "reminders.due",
            &json!({"now":"2026-11-10T10:00:00","lookback_minutes":60})
        )
        .unwrap()
        .as_array()
        .unwrap()
        .is_empty()
    );
    assert_eq!(
        s.call("reminders.history", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn reminder_invalid_inputs_are_rejected_and_no_lease_on_failure() {
    let mut s = db();
    enabled(&mut s);
    let future = s
        .call(
            "reminders.claim",
            &json!({"key":"nonexistent","now":"2026-11-10T12:00:00"}),
        )
        .unwrap();
    assert_eq!(future["claimed"], false);
    assert!(s.call("reminders.due", &json!({"now":"bad"})).is_err());
    assert!(
        s.call("reminders.upcoming", &json!({"forward_minutes":10081}))
            .is_err()
    );
    assert!(
        s.call(
            "reminders.snooze",
            &json!({"key":"x","now":"2026-11-10T09:00:00","until":"2026-11-09T09:00:00"})
        )
        .is_err()
    );
}
#[test]
fn snoozed_reminders_use_effective_trigger_even_when_original_date_outside_window() {
    let mut s = db();
    enabled(&mut s);
    let task = s
        .call(
            "tasks.create",
            &json!({"title":"Deferred obligation","due_date":"2026-11-09"}),
        )
        .unwrap();
    let key = format!("task:{}:2026-11-09", task["id"].as_str().unwrap());
    s.call(
        "reminders.snooze",
        &json!({"key":key,"now":"2026-11-09T09:00:00","until":"2026-11-12T11:00:00"}),
    )
    .unwrap();
    let early = s
        .call(
            "reminders.upcoming",
            &json!({"now":"2026-11-12T10:00:00","forward_minutes":120,"lookback_minutes":0}),
        )
        .unwrap();
    assert_eq!(
        early.as_array().unwrap().len(),
        1,
        "snoozed candidate must not be removed by original trigger date"
    );
    assert_eq!(early[0]["due_at"], "2026-11-12T11:00:00");
    let at_due = s
        .call(
            "reminders.due",
            &json!({"now":"2026-11-12T11:00:00","forward_minutes":0,"lookback_minutes":0}),
        )
        .unwrap();
    assert_eq!(at_due.as_array().unwrap().len(), 1);
    let overdue = s
        .call(
            "reminders.upcoming",
            &json!({"now":"2026-11-12T12:00:00","forward_minutes":0,"lookback_minutes":90}),
        )
        .unwrap();
    assert_eq!(
        overdue.as_array().unwrap().len(),
        1,
        "overdue snooze must remain recoverable"
    );
}
