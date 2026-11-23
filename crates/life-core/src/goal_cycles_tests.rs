use super::*;
fn store() -> Store {
    Store::open(std::env::temp_dir().join(format!("life-goal-cycle-{}.sqlite", id()))).unwrap()
}
#[test]
fn annual_and_quarterly_cycles_have_correct_boundaries() {
    let mut s = store();
    let year = s
        .call(
            "goal_cycles.create",
            &json!({"kind":"year","year":2027,"title":"Annual direction"}),
        )
        .unwrap();
    assert_eq!(year["cycle"]["starts_at"], "2027-01-01");
    assert_eq!(year["cycle"]["ends_at"], "2027-12-31");
    let q = s
        .call(
            "goal_cycles.create",
            &json!({"kind":"quarter","year":2027,"quarter":2}),
        )
        .unwrap();
    assert_eq!(q["cycle"]["period_key"], "2027-Q2");
    assert_eq!(q["cycle"]["starts_at"], "2027-04-01");
    assert_eq!(q["cycle"]["ends_at"], "2027-06-30");
    assert!(
        s.call(
            "goal_cycles.create",
            &json!({"kind":"quarter","year":2027,"quarter":2})
        )
        .is_err()
    );
    assert!(
        s.call(
            "goal_cycles.create",
            &json!({"kind":"quarter","year":2027,"quarter":5})
        )
        .is_err()
    );
}
#[test]
fn prioritization_enforces_three_active_goals_and_is_atomic() {
    let mut s = store();
    let cycle = s
        .call(
            "goal_cycles.create",
            &json!({"kind":"quarter","year":2027,"quarter":1}),
        )
        .unwrap();
    let key = cycle["cycle"]["id"].clone();
    let mut goals = Vec::new();
    for i in 0..4 {
        let goal = s
            .call("goals.create", &json!({"title":format!("Goal {i}")}))
            .unwrap();
        goals.push(goal["id"].clone());
    }
    let ranked = s
        .call(
            "goal_cycles.prioritize",
            &json!({"id":key,"goal_ids":[goals[0],goals[1],goals[2]]}),
        )
        .unwrap();
    assert_eq!(ranked["goals"].as_array().unwrap().len(), 3);
    assert_eq!(ranked["goals"][0]["rank"], 1);
    assert!(
        s.call(
            "goal_cycles.prioritize",
            &json!({"id":key,"goal_ids":goals})
        )
        .is_err()
    );
    assert!(
        s.call(
            "goal_cycles.prioritize",
            &json!({"id":key,"goal_ids":[goals[0],goals[0]]})
        )
        .is_err()
    );
    assert_eq!(
        s.call("goal_cycles.get", &json!({"id":key})).unwrap()["goals"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    s.call("goal_cycles.update", &json!({"id":key,"status":"complete"}))
        .unwrap();
    assert!(
        s.call(
            "goal_cycles.prioritize",
            &json!({"id":key,"goal_ids":[goals[0]]})
        )
        .is_err()
    );
}
#[test]
fn goal_cycle_list_and_archive_are_persistent() {
    let mut s = store();
    let a = s
        .call("goal_cycles.create", &json!({"kind":"year","year":2028}))
        .unwrap();
    assert_eq!(
        s.call("goal_cycles.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    s.call(
        "goal_cycles.update",
        &json!({"id":a["cycle"]["id"],"status":"archived"}),
    )
    .unwrap();
    assert_eq!(
        s.call("goal_cycles.list", &json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        s.call("goal_cycles.list", &json!({"include_archived":true}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
