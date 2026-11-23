use super::*;
fn db() -> Store {
    Store::open(std::env::temp_dir().join(format!("life-domain-test-{}.sqlite", id()))).unwrap()
}
#[test]
fn career_pipeline_records_progress_without_overwriting_newer_revision() {
    let mut s = db();
    let job=s.call("career.create",&json!({"employer":"Example Labs","role":"Platform Engineer","stage":"applied","location":"Singapore","salary_min":8000,"salary_max":12000,"currency":"SGD"})).unwrap();
    assert_eq!(
        s.call("career.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let updated=s.call("career.update",&json!({"id":job["id"],"stage":"interview","next_action":"Prepare technical assessment","next_action_on":"2026-11-11","expected_revision":1})).unwrap();
    assert_eq!(updated["revision"], 2);
    assert_eq!(updated["stage"], "interview");
    assert!(
        s.call(
            "career.update",
            &json!({"id":job["id"],"stage":"offer","expected_revision":1})
        )
        .is_err()
    );
    assert_eq!(
        s.call("career.get", &json!({"id":job["id"]})).unwrap()["stage"],
        "interview"
    );
    s.call("career.delete", &json!({"id":job["id"]})).unwrap();
    assert_eq!(
        s.call("career.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
    s.call("career.restore", &json!({"id":job["id"]})).unwrap();
    assert_eq!(
        s.call("career.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn learning_tracks_support_manual_progress_and_recorded_sessions() {
    let mut s = db();
    let course=s.call("learning.create",&json!({"title":"System design","provider":"Independent study","status":"active","progress":25,"target_hours":20})).unwrap();
    let session=s.call("learning.log",&json!({"track_id":course["id"],"minutes":60,"day":"2026-11-11","note":"Reviewed distributed data models"})).unwrap();
    assert_eq!(session["minutes"], 60);
    assert_eq!(
        s.call("learning.sessions", &json!({"track_id":course["id"]}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let updated = s
        .call("learning.update", &json!({"id":course["id"],"progress":50}))
        .unwrap();
    assert_eq!(updated["progress"].as_f64(), Some(50.0));
    assert!(
        s.call(
            "learning.update",
            &json!({"id":course["id"],"progress":101})
        )
        .is_err()
    );
    assert!(
        s.call(
            "learning.log",
            &json!({"track_id":course["id"],"minutes":0})
        )
        .is_err()
    );
}
#[test]
fn financial_reminders_keep_money_in_integer_minor_units() {
    let mut s = db();
    let bill=s.call("finance.create",&json!({"title":"Domain renewal","amount_minor":169900,"currency":"THB","due_date":"2026-12-01"})).unwrap();
    assert_eq!(bill["status"], "pending");
    assert_eq!(bill["amount_minor"], 169900);
    let paid = s
        .call("finance.update", &json!({"id":bill["id"],"status":"paid"}))
        .unwrap();
    assert_eq!(paid["status"], "paid");
    assert!(
        s.call(
            "finance.create",
            &json!({"title":"Invalid currency","currency":"BTCX"})
        )
        .is_err()
    );
    assert!(
        s.call(
            "finance.create",
            &json!({"title":"Invalid date","due_date":"2030-80-20"})
        )
        .is_err()
    );
}
#[test]
fn wellbeing_checkins_upsert_per_day_and_validate_ranges() {
    let mut s = db();
    s.call(
        "wellbeing.log",
        &json!({"day":"2026-11-10","energy":4,"mood":3,"sleep_hours":7.5}),
    )
    .unwrap();
    s.call(
        "wellbeing.log",
        &json!({"day":"2026-11-10","energy":2,"mood":4,"sleep_hours":6.0}),
    )
    .unwrap();
    let rows = s.call("wellbeing.list", &json!({})).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["energy"], 2);
    assert!(
        s.call("wellbeing.log", &json!({"energy":9,"mood":3}))
            .is_err()
    );
    assert!(
        s.call(
            "wellbeing.log",
            &json!({"energy":2,"mood":2,"sleep_hours":25})
        )
        .is_err()
    );
}
#[test]
fn personal_dashboard_rolls_up_domain_work() {
    let mut s = db();
    s.call("career.create",&json!({"employer":"Example","role":"Developer","stage":"interview","next_action_on":"2026-11-12"})).unwrap();
    s.call(
        "learning.create",
        &json!({"title":"Architecture","status":"active"}),
    )
    .unwrap();
    s.call(
        "finance.create",
        &json!({"title":"Insurance","amount_minor":12000,"due_date":"2026-11-15"}),
    )
    .unwrap();
    let dashboard = s.call("life_domains.dashboard", &json!({})).unwrap();
    assert_eq!(dashboard["upcoming_career"].as_array().unwrap().len(), 1);
    assert_eq!(dashboard["pending_finance"].as_array().unwrap().len(), 1);
    assert_eq!(dashboard["learning"][0]["count"], 1);
}
