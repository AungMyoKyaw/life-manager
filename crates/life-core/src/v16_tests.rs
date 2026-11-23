//! Tests for the schema v16 domain: personal life-area trackers, workflow
//! gaps, CSV import, keyboard shortcuts, integration accounts, remote MCP
//! clients and the security audit.
use super::*;

fn store() -> Store {
    Store::open(std::env::temp_dir().join(format!("life-v16-{}.sqlite", id()))).unwrap()
}

#[test]
fn interviews_savings_certifications_rest_and_administration_round_trip() {
    let mut s = store();
    let interview = s
        .call(
            "interviews.create",
            &json!({"title":"Technical interview","starts_at":"2026-04-01T10:00:00","location":"Video","preparation":"Architecture"}),
        )
        .unwrap();
    assert_eq!(interview["status"], "scheduled");
    assert_eq!(
        s.call("interviews.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        s.call(
            "interviews.create",
            &json!({"title":"Bad","starts_at":"2026-04-02T10:00:00","ends_at":"2026-04-01T10:00:00"}),
        )
        .is_err()
    );
    s.call(
        "interviews.update",
        &json!({"id":interview["id"],"status":"done"}),
    )
    .unwrap();
    assert!(
        s.call("interviews.delete", &json!({"id":interview["id"]}))
            .is_ok()
    );

    let saving = s
        .call(
            "savings_goals.create",
            &json!({"title":"Emergency fund","target_minor":500000,"currency":"SGD","due_date":"2026-12-01"}),
        )
        .unwrap();
    assert_eq!(saving["target_minor"], 500000);
    assert!(
        s.call(
            "savings_goals.create",
            &json!({"title":"Bad","target_minor":-1}),
        )
        .is_err()
    );

    let certification = s
        .call(
            "certifications.create",
            &json!({"title":"CKA","provider":"Linux Foundation","deadline_date":"2026-05-01"}),
        )
        .unwrap();
    assert_eq!(certification["status"], "planned");
    assert!(
        s.call(
            "certifications.create",
            &json!({"title":"Bad","deadline_date":"not-a-date"}),
        )
        .is_err()
    );

    let rest = s
        .call(
            "rest_intentions.create",
            &json!({"title":"Sleep","kind":"sleep","starts_at":"22:00","ends_at":"07:00","days":[0,1,2]}),
        )
        .unwrap();
    assert_eq!(rest["kind"], "sleep");
    assert!(
        s.call(
            "rest_intentions.create",
            &json!({"title":"Bad","kind":"napping","days":[9]}),
        )
        .is_err()
    );

    let item = s
        .call(
            "administration.create",
            &json!({"title":"Passport renewal","kind":"renewal","renewal_date":"2027-01-01","amount_minor":5000}),
        )
        .unwrap();
    assert_eq!(item["reminder_days"], 14);
    assert!(
        s.call(
            "administration.create",
            &json!({"title":"Bad","reminder_days":900}),
        )
        .is_err()
    );

    let overview = s.call("personal.overview", &json!({})).unwrap();
    assert!(overview["source"].as_str().unwrap().contains("SQLite"));
    assert_eq!(overview["savings_goals"].as_array().unwrap().len(), 1);
}

#[test]
fn overdue_decisions_list_and_apply() {
    let mut s = store();
    let task = s
        .call(
            "tasks.create",
            &json!({"title":"Late work","due_date":"2020-01-01","priority":1}),
        )
        .unwrap();
    let decisions = s.call("overdue.decisions", &json!({})).unwrap();
    assert_eq!(decisions["count"], 1);
    let snoozed = s
        .call(
            "overdue.decide",
            &json!({"id":task["id"],"decision":"snooze","date":"2026-04-02"}),
        )
        .unwrap();
    assert_eq!(snoozed["decision"], "snooze");
    let updated = s.call("tasks.get", &json!({"id":task["id"]})).unwrap();
    assert_eq!(updated["due_date"], "2026-04-02");

    s.call(
        "overdue.decide",
        &json!({"id":task["id"],"decision":"reprioritise","priority":4}),
    )
    .unwrap();
    let updated = s.call("tasks.get", &json!({"id":task["id"]})).unwrap();
    assert_eq!(updated["priority"], 4);

    assert!(
        s.call(
            "overdue.decide",
            &json!({"id":task["id"],"decision":"invent"}),
        )
        .is_err()
    );
}

#[test]
fn entity_timeline_reports_recorded_activity() {
    let mut s = store();
    let task = s.call("tasks.create", &json!({"title":"Tracked"})).unwrap();
    let timeline = s
        .call("entity.timeline", &json!({"kind":"tasks","id":task["id"]}))
        .unwrap();
    assert!(timeline["count"].as_i64().unwrap() >= 1);
    assert!(timeline["source"].as_str().unwrap().contains("SQLite"));
    assert!(s.call("entity.timeline", &json!({"kind":"tasks"})).is_err());
}

#[test]
fn capture_parser_understands_dates_urgency_and_tags() {
    let mut s = store();
    let draft = s
        .call(
            "capture.parse",
            &json!({"text":"Draft the report tomorrow #work"}),
        )
        .unwrap();
    assert_eq!(draft["title"], "Draft the report");
    assert_eq!(draft["status"], "inbox");
    assert_eq!(draft["priority"], 3);
    assert!(draft["due_date"].as_str().is_some());
    assert_eq!(draft["tags"][0], "work");

    let urgent = s
        .call("capture.parse", &json!({"text":"urgent fix"}))
        .unwrap();
    assert_eq!(urgent["priority"], 1);
    assert_eq!(urgent["title"], "fix");
    assert!(s.call("capture.parse", &json!({"text":"   "})).is_err());
}

#[test]
fn csv_import_previews_then_applies_and_detects_duplicates() {
    let mut s = store();
    let csv = "title,due_date,priority\nShip the release,2026-05-01,1\nShip the release,2026-05-01,1\n,2026-05-02,2\n";
    let preview = s
        .call("csv.import.preview", &json!({"content":csv}))
        .unwrap();
    assert_eq!(preview["valid"].as_array().unwrap().len(), 1);
    assert_eq!(preview["duplicates"].as_array().unwrap().len(), 1);
    assert_eq!(preview["invalid"].as_array().unwrap().len(), 1);
    assert_eq!(preview["total"], 3);
    assert_eq!(
        s.call("tasks.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );

    assert!(s.call("csv.import.apply", &json!({"content":csv})).is_err());
    let applied = s
        .call(
            "csv.import.apply",
            &json!({"content":csv,"confirm":true,"allow_invalid":true}),
        )
        .unwrap();
    assert_eq!(applied["created_count"], 1);
    assert_eq!(
        s.call("tasks.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // The same file again is entirely duplicate.
    let second = s
        .call("csv.import.preview", &json!({"content":csv}))
        .unwrap();
    assert_eq!(second["valid"].as_array().unwrap().len(), 0);
    assert!(!second["duplicates"].as_array().unwrap().is_empty());
}

#[test]
fn shortcuts_are_seeded_and_conflicts_are_refused() {
    let mut s = store();
    let shortcuts = s.call("shortcuts.list", &json!({})).unwrap();
    assert!(shortcuts.as_array().unwrap().len() >= 8);
    s.call(
        "shortcuts.update",
        &json!({"action":"command_palette","keys":"Meta+P"}),
    )
    .unwrap();
    assert!(
        s.call(
            "shortcuts.update",
            &json!({"action":"quick_capture","keys":"Meta+P"}),
        )
        .is_err()
    );
    s.call("shortcuts.reset", &json!({})).unwrap();
    let reset = s.call("shortcuts.list", &json!({})).unwrap();
    assert!(
        reset
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["keys"] == "Meta+K")
    );
}

#[test]
fn integrations_report_what_is_missing_and_never_invent_a_state() {
    let mut s = store();
    let listed = s.call("integrations.list", &json!({})).unwrap();
    let providers: Vec<String> = listed
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["provider"].as_str().unwrap_or("").to_string())
        .collect();
    for expected in ["google_calendar", "apple_calendar", "caldav", "github"] {
        assert!(providers.contains(&expected.to_string()), "{expected}");
    }
    let configured = s
        .call(
            "integrations.configure",
            &json!({"provider":"google_calendar"}),
        )
        .unwrap();
    assert_eq!(configured["configured"], false);
    assert!(configured["missing"].as_array().unwrap().len() >= 2);
    assert!(
        s.call("integrations.configure", &json!({"provider":"telepathy"}))
            .is_err()
    );
    s.call(
        "integrations.status",
        &json!({"provider":"google_calendar","status":"disconnected"}),
    )
    .unwrap();
    let after = s.call("integrations.list", &json!({})).unwrap();
    assert!(
        after
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["provider"] == "google_calendar" && row["status"] == "disconnected")
    );
}

#[test]
fn remote_clients_are_hashed_and_can_be_revoked() {
    let mut s = store();
    let token = "lm_0123456789abcdef0123456789";
    let client = s
        .call(
            "remote_clients.create",
            &json!({"name":"Test agent","token":token}),
        )
        .unwrap();
    assert!(!client.to_string().contains(token));
    assert!(
        s.call(
            "remote_clients.create",
            &json!({"name":"Short","token":"abc"}),
        )
        .is_err()
    );
    assert_eq!(
        s.remote_client_authorised(token).unwrap().as_deref(),
        Some("Test agent")
    );
    s.call("remote_clients.revoke", &json!({"id":client["id"]}))
        .unwrap();
    assert!(s.remote_client_authorised(token).unwrap().is_none());
    assert!(
        s.remote_client_authorised("lm_wrong_wrong_wrong_wrong")
            .unwrap()
            .is_none()
    );
}

#[test]
fn secrets_are_redacted_before_they_reach_the_ledger() {
    let s = store();
    let payload =
        json!({"title":"Sync","access_token":"secret-value","nested":{"client_secret":"another"}});
    let redacted = s.redact(&payload);
    assert_eq!(redacted["access_token"], "[redacted]");
    assert_eq!(redacted["nested"]["client_secret"], "[redacted]");
    assert_eq!(redacted["title"], "Sync");
}

#[test]
fn security_audit_records_events() {
    let mut s = store();
    s.call(
        "security.audit.log",
        &json!({"event":"backup.restore","detail":"Restored a verified snapshot"}),
    )
    .unwrap();
    let log = s.call("security.audit.list", &json!({})).unwrap();
    assert_eq!(log.as_array().unwrap().len(), 1);
    assert_eq!(log[0]["event"], "backup.restore");
}

#[test]
fn system_status_and_diagnostics_report_the_live_database() {
    let mut s = store();
    let status = s.call("system.status", &json!({})).unwrap();
    assert_eq!(status["schema_version"], SCHEMA_VERSION);
    assert_eq!(status["integrity"], "ok");
    assert_eq!(status["up_to_date"], true);
    let report = s.call("diagnostics.report", &json!({})).unwrap();
    assert!(report["tables"].as_array().unwrap().len() >= 10);
    assert!(report["source"].as_str().unwrap().contains("SQLite"));
}

#[test]
fn planning_session_records_three_priorities() {
    let mut s = store();
    let session = s
        .call(
            "planning.session",
            &json!({"date":"2026-04-01","priorities":["Write the report","Review the plan","Ship it"]}),
        )
        .unwrap();
    assert_eq!(session["priorities"].as_array().unwrap().len(), 3);
    assert!(
        s.call(
            "planning.session",
            &json!({"date":"2026-04-01","priorities":["a","b","c","d"]}),
        )
        .is_err()
    );
    let stored = s
        .call("planning.session.get", &json!({"date":"2026-04-01"}))
        .unwrap();
    assert_eq!(stored["session"]["priorities"].as_array().unwrap().len(), 3);
}
