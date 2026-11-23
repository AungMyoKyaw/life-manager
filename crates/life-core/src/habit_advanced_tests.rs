use super::*;
fn sample() -> Store {
    Store::open(std::env::temp_dir().join(format!("life-habits-{}.sqlite", id()))).unwrap()
}
#[test]
fn flexible_weekly_goal_and_voluntary_rest() {
    let mut s = sample();
    let h = s
        .call(
            "habits.create",
            &json!({"title":"Exercise","frequency":"weekly","target":1}),
        )
        .unwrap();
    let configured = s
        .call(
            "habits.configure",
            &json!({"id":h["id"],"quota_period":"weekly","weekly_quota":3,"rest_days":[5,6]}),
        )
        .unwrap();
    assert_eq!(configured["weekly_quota"], 3);
    let a = s
        .call(
            "habits.log",
            &json!({"habit_id":h["id"],"day":"2026-11-09","value":1}),
        )
        .unwrap();
    assert_eq!(a["value"], 1);
    let rest = s
        .call(
            "habits.skip",
            &json!({"habit_id":h["id"],"day":"2026-11-10","status":"rest","note":"Recovery"}),
        )
        .unwrap();
    assert_eq!(rest["status"], "rest");
    let values = s
        .call("habits.weekly", &json!({"day":"2026-11-12"}))
        .unwrap();
    let item = values
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == h["id"])
        .unwrap();
    assert_eq!(item["completed_days"], 1);
    assert_eq!(item["rest_days_taken"], 1);
    assert_eq!(item["quota_for_period"], 3);
    assert_eq!(item["fulfilled"], false);
    s.call(
        "habits.log",
        &json!({"habit_id":h["id"],"day":"2026-11-10","value":2}),
    )
    .unwrap();
    let item = s
        .call("habits.weekly", &json!({"day":"2026-11-12"}))
        .unwrap();
    assert_eq!(item[0]["completed_days"], 2);
    assert_eq!(item[0]["rest_days_taken"], 0);
}
#[test]
fn invalid_quotas_and_skip_states_do_not_mutate() {
    let mut s = sample();
    let h = s.call("habits.create", &json!({"title":"Read"})).unwrap();
    assert!(
        s.call(
            "habits.configure",
            &json!({"id":h["id"],"quota_period":"weekly","weekly_quota":7,"rest_days":[6]})
        )
        .is_err()
    );
    assert!(
        s.call(
            "habits.configure",
            &json!({"id":h["id"],"quota_period":"weekly","weekly_quota":2,"rest_days":[6,6]})
        )
        .is_err()
    );
    assert!(
        s.call(
            "habits.skip",
            &json!({"habit_id":h["id"],"day":"2026-11-09","status":"done"})
        )
        .is_err()
    );
    let row = s
        .call("habits.weekly", &json!({"day":"2026-11-09"}))
        .unwrap();
    assert_eq!(row[0]["completed_days"], 0);
    assert_eq!(row[0]["quota_period"], "daily");
}
