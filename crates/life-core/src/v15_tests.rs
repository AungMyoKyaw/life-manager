//! Tests for the schema v15 domain: recurrence exceptions, calendar sources,
//! planning rules, focus modes, executable views, insight reports and agent
//! policy.

use super::*;

fn disposable() -> Store {
    Store::open(std::env::temp_dir().join(format!("life-v15-{}.sqlite", id()))).unwrap()
}

#[test]
fn recurrence_exceptions_skip_and_reschedule_single_occurrences() {
    let mut s = disposable();
    let task = s
        .call(
            "tasks.create",
            &json!({"title":"Daily reading","due_date":"2026-03-02","recurrence":"daily"}),
        )
        .unwrap();
    let task_id = task["id"].as_str().unwrap().to_string();

    let window = s
        .call(
            "recurrence.occurrences",
            &json!({"task_id":task_id,"from":"2026-03-02","to":"2026-03-05"}),
        )
        .unwrap();
    assert_eq!(window["count"], 4);
    assert_eq!(window["occurrences"][0]["state"], "scheduled");

    s.call(
        "recurrence.exceptions.create",
        &json!({"task_id":task_id,"occurrence_date":"2026-03-03","action":"skip","reason":"travel"}),
    )
    .unwrap();
    let after = s
        .call(
            "recurrence.occurrences",
            &json!({"task_id":task_id,"from":"2026-03-02","to":"2026-03-05"}),
        )
        .unwrap();
    assert_eq!(after["occurrences"][1]["state"], "skipped");

    s.call(
        "recurrence.exceptions.create",
        &json!({"task_id":task_id,"occurrence_date":"2026-03-04","action":"reschedule","moved_to":"2026-03-09"}),
    )
    .unwrap();
    let moved = s
        .call(
            "recurrence.occurrences",
            &json!({"task_id":task_id,"from":"2026-03-02","to":"2026-03-05"}),
        )
        .unwrap();
    assert_eq!(moved["occurrences"][2]["state"], "moved");
    assert_eq!(moved["occurrences"][2]["moved_to"], "2026-03-09");

    let listed = s
        .call("recurrence.exceptions.list", &json!({"task_id":task_id}))
        .unwrap();
    assert_eq!(listed.as_array().unwrap().len(), 2);

    s.call(
        "recurrence.exceptions.delete",
        &json!({"task_id":task_id,"occurrence_date":"2026-03-03"}),
    )
    .unwrap();
    let remaining = s
        .call("recurrence.exceptions.list", &json!({"task_id":task_id}))
        .unwrap();
    assert_eq!(remaining.as_array().unwrap().len(), 1);
}

#[test]
fn recurrence_exceptions_reject_invalid_input() {
    let mut s = disposable();
    let task = s
        .call(
            "tasks.create",
            &json!({"title":"Weekly review","due_date":"2026-03-02","recurrence":"weekly"}),
        )
        .unwrap();
    let task_id = task["id"].as_str().unwrap().to_string();

    assert!(
        s.call(
            "recurrence.exceptions.create",
            &json!({"task_id":task_id,"occurrence_date":"not-a-date","action":"skip"}),
        )
        .is_err()
    );
    assert!(
        s.call(
            "recurrence.exceptions.create",
            &json!({"task_id":task_id,"occurrence_date":"2026-03-09","action":"invent"}),
        )
        .is_err()
    );
    assert!(
        s.call(
            "recurrence.exceptions.create",
            &json!({"task_id":task_id,"occurrence_date":"2026-03-09","action":"reschedule"}),
        )
        .is_err()
    );
    assert!(
        s.call(
            "recurrence.exceptions.create",
            &json!({"task_id":"missing","occurrence_date":"2026-03-09","action":"skip"}),
        )
        .is_err()
    );
}

#[test]
fn calendar_sources_are_listed_and_removed() {
    let mut s = disposable();
    let source = s
        .call(
            "calendar.sources.create",
            &json!({"name":"Imported ICS","kind":"ics","color":"#8C9E90"}),
        )
        .unwrap();
    assert_eq!(source["read_only"], 1);
    assert!(source["read_only"].is_i64());
    let listed = s.call("calendar.sources.list", &json!({})).unwrap();
    assert_eq!(listed.as_array().unwrap().len(), 1);
    let updated = s
        .call(
            "calendar.sources.update",
            &json!({"id":source["id"],"name":"Renamed","read_only":false}),
        )
        .unwrap();
    assert_eq!(updated["name"], "Renamed");
    assert_eq!(updated["read_only"], 0);
    s.call("calendar.sources.delete", &json!({"id":source["id"]}))
        .unwrap();
    assert_eq!(
        s.call("calendar.sources.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert!(
        s.call(
            "calendar.sources.create",
            &json!({"name":"Bad","kind":"telepathy"}),
        )
        .is_err()
    );
}

#[test]
fn protected_time_and_weekly_budgets_are_persisted() {
    let mut s = disposable();
    let block = s
        .call(
            "protected.create",
            &json!({"title":"Deep work","weekday":0,"start_time":"09:00","end_time":"11:00"}),
        )
        .unwrap();
    assert_eq!(block["weekday"], 0);
    assert!(
        s.call(
            "protected.create",
            &json!({"title":"Bad","weekday":9,"start_time":"09:00","end_time":"11:00"}),
        )
        .is_err()
    );
    assert!(
        s.call(
            "protected.create",
            &json!({"title":"Bad","weekday":0,"start_time":"11:00","end_time":"09:00"}),
        )
        .is_err()
    );
    let area = s
        .call("areas.create", &json!({"name":"Budget Area"}))
        .unwrap();
    let area_id = area["id"].as_str().unwrap().to_string();
    s.call(
        "time_budgets.set",
        &json!({"scope_kind":"area","scope_id":area_id,"period":"week","minutes":600}),
    )
    .unwrap();
    let budgets = s.call("time_budgets.list", &json!({})).unwrap();
    assert_eq!(budgets.as_array().unwrap().len(), 1);
    let report = s.call("time_budgets.report", &json!({})).unwrap();
    assert!(!report["areas"].as_array().unwrap().is_empty());
    assert!(report["source"].as_str().unwrap().contains("SQLite"));
    assert!(
        s.call(
            "time_budgets.set",
            &json!({"scope_kind":"area","period":"week","minutes":600}),
        )
        .is_err()
    );
}

#[test]
fn carryover_lists_and_moves_unfinished_work() {
    let mut s = disposable();
    let task = s
        .call(
            "tasks.create",
            &json!({"title":"Unfinished","due_date":"2026-03-01","planned_date":"2026-03-01"}),
        )
        .unwrap();
    let carry = s
        .call(
            "planning.carryover",
            &json!({"from":"2026-03-02","to":"2026-03-09"}),
        )
        .unwrap();
    assert_eq!(
        carry["count"], 1,
        "unfinished work before the window is carried over"
    );
    s.call(
        "planning.carryover.apply",
        &json!({"ids":[task["id"]],"target_date":"2026-03-10"}),
    )
    .unwrap();
    let moved = s.call("tasks.get", &json!({"id":task["id"]})).unwrap();
    assert_eq!(moved["planned_date"], "2026-03-10");
    assert!(
        s.call(
            "planning.carryover.apply",
            &json!({"ids":[task["id"]],"target_date":"nope"}),
        )
        .is_err()
    );
}

#[test]
fn focus_modes_presets_manual_entries_and_trends() {
    let mut s = disposable();
    let preset = s
        .call(
            "focus.presets.create",
            &json!({"name":"Deep pomodoro","focus_minutes":50,"break_minutes":10,"cycles":3}),
        )
        .unwrap();
    assert_eq!(preset["focus_minutes"], 50);
    assert!(
        s.call(
            "focus.presets.create",
            &json!({"name":"Bad","focus_minutes":0}),
        )
        .is_err()
    );

    let session = s
        .call(
            "focus.start",
            &json!({"mode":"pomodoro","preset_id":preset["id"]}),
        )
        .unwrap();
    assert_eq!(session["mode"], "pomodoro");
    assert!(s.call("focus.start", &json!({"mode":"pomodoro"})).is_err());
    s.call("focus.stop", &json!({"id":session["id"]})).unwrap();

    let manual = s
        .call(
            "focus.manual",
            &json!({"minutes":45,"day":"2026-03-02","reason":"offline work"}),
        )
        .unwrap();
    assert_eq!(manual["mode"], "manual");
    assert_eq!(manual["duration_seconds"], 2700);
    assert!(s.call("focus.manual", &json!({"minutes":0})).is_err());

    let trends = s
        .call(
            "focus.trends",
            &json!({"from":"2026-03-01","to":"2026-03-05"}),
        )
        .unwrap();
    assert!(trends["source"].as_str().unwrap().contains("SQLite"));
}

#[test]
fn saved_views_execute_and_saved_searches_run() {
    let mut s = disposable();
    s.call(
        "tasks.create",
        &json!({"title":"Urgent work","priority":1,"status":"todo"}),
    )
    .unwrap();
    s.call(
        "tasks.create",
        &json!({"title":"Relaxed work","priority":4,"status":"todo"}),
    )
    .unwrap();
    let view = s
        .call(
            "saved_views.create",
            &json!({"name":"Urgent","query":{"priority":1},"entity":"tasks"}),
        )
        .unwrap();
    let run = s
        .call("saved_views.run", &json!({"id":view["id"]}))
        .unwrap();
    assert_eq!(run["count"], 1);
    assert_eq!(run["results"][0]["title"], "Urgent work");

    assert!(
        s.call(
            "saved_views.create",
            &json!({"name":"Bad","query":{"unknown_field":1}}),
        )
        .is_err()
    );

    let search = s
        .call(
            "saved_searches.create",
            &json!({"name":"All open","entity":"all","query":{"status":"todo"}}),
        )
        .unwrap();
    let results = s
        .call("saved_searches.run", &json!({"id":search["id"]}))
        .unwrap();
    assert_eq!(results["count"], 2);
}

#[test]
fn backlinks_report_both_directions() {
    let mut s = disposable();
    let note = s
        .call("notes.create", &json!({"title":"Research note"}))
        .unwrap();
    let task = s
        .call("tasks.create", &json!({"title":"Implement it"}))
        .unwrap();
    s.call(
        "links.add",
        &json!({
            "source_kind":"notes","source_id":note["id"],
            "target_kind":"tasks","target_id":task["id"],"relation":"reference"
        }),
    )
    .unwrap();
    let from_task = s
        .call("links.backlinks", &json!({"kind":"tasks","id":task["id"]}))
        .unwrap();
    assert_eq!(from_task["count"], 1);
    assert_eq!(from_task["links"][0]["direction"], "incoming");
    assert_eq!(from_task["links"][0]["title"], "Research note");
    let from_note = s
        .call("links.backlinks", &json!({"kind":"notes","id":note["id"]}))
        .unwrap();
    assert_eq!(from_note["links"][0]["direction"], "outgoing");
}

#[test]
fn insight_reports_state_their_source() {
    let mut s = disposable();
    let area = s
        .call("areas.create", &json!({"name":"Report Area"}))
        .unwrap();
    let area_id = area["id"].as_str().unwrap().to_string();
    s.call(
        "tasks.create",
        &json!({"title":"Done work","area_id":area_id,"estimate_minutes":60,"status":"done","completed_at":"2026-03-02T10:00:00Z","planned_date":"2026-03-02"}),
    )
    .unwrap();
    for action in [
        "reports.plan_vs_actual",
        "reports.goal_achievement",
        "reports.area_distribution",
        "reports.journal_export",
        "goals.limit",
    ] {
        let result = s
            .call(action, &json!({"from":"2026-03-01","to":"2026-03-05"}))
            .unwrap();
        assert!(
            result["source"].as_str().unwrap().contains("SQLite"),
            "{action} must name its source"
        );
    }
    let limit = s.call("goals.limit", &json!({"limit":0})).unwrap();
    assert_eq!(limit["limit"], 1);
}

#[test]
fn agent_permissions_deny_disallowed_actions() {
    let mut s = disposable();
    // Without any rule the actor is unrestricted.
    s.call(
        "tasks.create",
        &json!({"title":"Allowed by default","actor":"helper"}),
    )
    .unwrap();
    s.call(
        "agents.permissions.set",
        &json!({"target_actor":"helper","action_pattern":"tasks.*","allowed":true}),
    )
    .unwrap();
    s.call(
        "tasks.create",
        &json!({"title":"Still allowed","actor":"helper"}),
    )
    .unwrap();
    s.call(
        "agents.permissions.set",
        &json!({"target_actor":"helper","action_pattern":"tasks.create","allowed":false}),
    )
    .unwrap();
    assert!(
        s.call("tasks.create", &json!({"title":"Denied","actor":"helper"}))
            .is_err()
    );
    // A read that is not matched by any rule is refused for a restricted actor.
    assert!(s.call("projects.list", &json!({"actor":"helper"})).is_err());
    let rules = s
        .call("agents.permissions.list", &json!({"actor":"helper"}))
        .unwrap();
    assert_eq!(rules.as_array().unwrap().len(), 2);
}

#[test]
fn agent_proposals_are_reviewed_before_they_apply() {
    let mut s = disposable();
    let proposal = s
        .call(
            "agents.proposals.create",
            &json!({"action":"tasks.create","payload":{"title":"Proposed task"},"actor":"helper"}),
        )
        .unwrap();
    assert_eq!(proposal["state"], "pending");
    assert!(
        s.call("tasks.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["title"] != "Proposed task")
    );
    let approved = s
        .call(
            "agents.proposals.approve",
            &json!({"id":proposal["id"],"reason":"looks good"}),
        )
        .unwrap();
    assert_eq!(approved["state"], "approved");
    assert!(
        s.call("tasks.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["title"] == "Proposed task")
    );
    assert!(
        s.call("agents.proposals.approve", &json!({"id":proposal["id"]}),)
            .is_err()
    );

    let second = s
        .call(
            "agents.proposals.create",
            &json!({"action":"tasks.create","payload":{"title":"Rejected task"}}),
        )
        .unwrap();
    let rejected = s
        .call(
            "agents.proposals.reject",
            &json!({"id":second["id"],"reason":"not now"}),
        )
        .unwrap();
    assert_eq!(rejected["state"], "rejected");
    assert!(
        s.call("tasks.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["title"] != "Rejected task")
    );
}

#[test]
fn automation_schedules_are_due_once_and_recorded() {
    let mut s = disposable();
    let rule = s
        .call(
            "automations.create",
            &json!({"title":"Capture note","action":"notes.create","arguments":{"title":"Scheduled note"}}),
        )
        .unwrap();
    s.call(
        "automations.schedule.set",
        &json!({"rule_id":rule["id"],"kind":"daily","at_time":"00:00"}),
    )
    .unwrap();
    let due = s.call("automations.schedule.due", &json!({})).unwrap();
    assert_eq!(due["count"], 1);
    let ran = s.call("automations.schedule.run_due", &json!({})).unwrap();
    assert_eq!(ran["count"], 1);
    assert_eq!(ran["ran"][0]["ok"], true);
    let after = s.call("automations.schedule.due", &json!({})).unwrap();
    assert_eq!(after["count"], 0);
    assert!(
        s.call(
            "automations.schedule.set",
            &json!({"rule_id":rule["id"],"kind":"weekly","at_time":"08:00"}),
        )
        .is_err()
    );
}
