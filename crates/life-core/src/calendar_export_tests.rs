use super::*;
#[test]
fn ics_export_is_rfc5545_readable_and_roundtrip_imports() {
    let mut s =
        Store::open(std::env::temp_dir().join(format!("life-ical-roundtrip-{}.sqlite", id())))
            .unwrap();
    s.call("events.create",&json!({"title":"Review, plan","description":"A; B\nC","start_at":"2026-11-12T09:00:00","end_at":"2026-11-12T10:00:00"})).unwrap();
    s.call("events.create",&json!({"title":"Day off","start_at":"2026-11-13T00:00:00","end_at":"2026-11-14T00:00:00","all_day":true})).unwrap();
    let result = s.call("calendar.export_ics", &json!({})).unwrap();
    let text = result["content"].as_str().unwrap();
    assert!(text.contains("DTSTART:20261112T090000"), "{text}");
    assert!(text.contains("DTSTART;VALUE=DATE:20261113"), "{text}");
    assert!(text.contains("SUMMARY:Review\\, plan"), "{text}");
    let mut next =
        Store::open(std::env::temp_dir().join(format!("life-ical-roundtrip-{}.sqlite", id())))
            .unwrap();
    let preview = next
        .call(
            "calendar.import.preview",
            &json!({"content":text,"source":"roundtrip.ics"}),
        )
        .unwrap();
    assert_eq!(preview["valid"].as_array().unwrap().len(), 2, "{preview}");
    let applied = next
        .call(
            "calendar.import.apply",
            &json!({"content":text,"source":"roundtrip.ics","confirm":true}),
        )
        .unwrap();
    assert_eq!(applied["created"].as_array().unwrap().len(), 2);
    let rows = next.call("events.list", &json!({})).unwrap();
    assert!(rows.as_array().unwrap().iter().all(|r| r["locked"] == 1));
}
