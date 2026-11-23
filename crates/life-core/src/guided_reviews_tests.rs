use super::*;
fn store() -> Store {
    Store::open(std::env::temp_dir().join(format!("life-guided-review-{}.sqlite", id()))).unwrap()
}
#[test]
fn guided_review_preserves_draft_then_generates_canonical_review() {
    let mut store = store();
    let p = json!({"kind":"weekly","day":"2026-11-09"});
    let started = store.call("review_guides.start", &p).unwrap();
    assert_eq!(started["state"], "draft");
    assert_eq!(started["kind"], "weekly");
    let replay = store.call("review_guides.start", &p).unwrap();
    assert_eq!(started["id"], replay["id"]);
    let key = started["id"].clone();
    let draft = store
        .call(
            "review_guides.update",
            &json!({"id":key,"answers":{
                "wins":"Built the implementation","adjustments":"Protect more focused time"
            }}),
        )
        .unwrap();
    let contents: Value = serde_json::from_str(draft["answers_json"].as_str().unwrap()).unwrap();
    assert_eq!(contents["wins"], "Built the implementation");
    let finished = store
        .call("review_guides.complete", &json!({"id":key}))
        .unwrap();
    assert_eq!(finished["guided"]["state"], "complete");
    let reviews = store
        .call("reviews.list", &json!({"period":"weekly"}))
        .unwrap();
    assert_eq!(reviews.as_array().unwrap().len(), 1);
    assert!(
        reviews[0]["content"]
            .as_str()
            .unwrap()
            .contains("Built the implementation")
    );
    assert!(
        reviews[0]["content"]
            .as_str()
            .unwrap()
            .contains("Metrics are snapshots")
    );
    assert!(
        store
            .call("review_guides.complete", &json!({"id":key}))
            .is_err()
    );
    assert!(
        store
            .call(
                "review_guides.update",
                &json!({"id":key,"answers":{"wins":"Overwrite"}})
            )
            .is_err()
    );
}
#[test]
fn morning_and_evening_are_separate_and_validate_answer_keys() {
    let mut store = store();
    let day = "2026-11-11";
    let morning = store
        .call("review_guides.start", &json!({"kind":"morning","day":day}))
        .unwrap();
    let evening = store
        .call("review_guides.start", &json!({"kind":"evening","day":day}))
        .unwrap();
    assert_ne!(morning["id"], evening["id"]);
    assert!(
        store
            .call(
                "review_guides.update",
                &json!({"id":morning["id"],"answers":{"unexpected":"hi"}})
            )
            .is_err()
    );
    assert!(
        store
            .call(
                "review_guides.update",
                &json!({"id":morning["id"],"answers":{"intention":12}})
            )
            .is_err()
    );
    assert!(
        store
            .call("review_guides.complete", &json!({"id":morning["id"]}))
            .is_err()
    );
    let prompt = store.call("review_guides.prompts", &json!({})).unwrap();
    assert_eq!(prompt.as_array().unwrap().len(), 4);
}
#[test]
fn monthly_metrics_have_provenance_and_invalid_dates_rejected() {
    let mut store = store();
    let start = store
        .call(
            "review_guides.start",
            &json!({"kind":"monthly","day":"2026-11-09"}),
        )
        .unwrap();
    let metrics: Value = serde_json::from_str(start["metrics_json"].as_str().unwrap()).unwrap();
    assert_eq!(metrics["from"], "2026-11-01");
    assert!(metrics["source"].as_str().unwrap().contains("SQLite"));
    assert!(
        store
            .call(
                "review_guides.start",
                &json!({"kind":"weekly","day":"2026-22-09"})
            )
            .is_err()
    );
    assert!(
        store
            .call("review_guides.start", &json!({"kind":"bogus"}))
            .is_err()
    );
}
