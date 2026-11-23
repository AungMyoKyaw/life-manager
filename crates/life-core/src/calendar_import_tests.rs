use super::*;
fn db() -> Store {
    Store::open(std::env::temp_dir().join(format!("life-calendar-import-{}.sqlite", id()))).unwrap()
}
fn fixture() -> String {
    "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:one@example.test\r\nSUMMARY:Review\\, plan\r\nDTSTART:20261110T090000\r\nDTEND:20261110T100000\r\nDESCRIPTION:Meeting details\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:two@example.test\r\nSUMMARY:Time off\r\nDTSTART;VALUE=DATE:20261112\r\nDTEND;VALUE=DATE:20261113\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n".to_string()
}
#[test]
fn calendar_import_preview_and_confirm_protect_imported_events() {
    let mut s = db();
    let content = fixture();
    let input = json!({"source":"sample.ics","content":content});
    let preview = s.call("calendar.import.preview", &input).unwrap();
    assert_eq!(preview["valid"].as_array().unwrap().len(), 2);
    assert_eq!(preview["valid"][0]["title"], "Review, plan");
    assert_eq!(preview["valid"][1]["all_day"], true);
    assert!(s.call("calendar.import.apply", &input).is_err());
    let applied = s
        .call(
            "calendar.import.apply",
            &json!({"source":"sample.ics","content":content,"confirm":true}),
        )
        .unwrap();
    assert_eq!(applied["created"].as_array().unwrap().len(), 2);
    let found = s.call("events.list", &json!({})).unwrap();
    assert_eq!(found.as_array().unwrap().len(), 2);
    assert!(found.as_array().unwrap().iter().all(|v| v["locked"] == 1));
    let second = s.call("calendar.import.preview", &input).unwrap();
    assert_eq!(second["duplicates"].as_array().unwrap().len(), 2);
    let repeated = s
        .call(
            "calendar.import.apply",
            &json!({"source":"sample.ics","content":content,"confirm":true}),
        )
        .unwrap();
    assert_eq!(repeated["created"].as_array().unwrap().len(), 0);
}
#[test]
fn unsupported_recurrence_requires_explicit_skip() {
    let mut s = db();
    let content = fixture().replace(
        "DESCRIPTION:Meeting details",
        "RRULE:FREQ=WEEKLY\nDESCRIPTION:Meeting details",
    );
    let preview = s
        .call("calendar.import.preview", &json!({"content":content}))
        .unwrap();
    assert_eq!(preview["invalid"].as_array().unwrap().len(), 1);
    assert_eq!(preview["valid"].as_array().unwrap().len(), 1);
    assert!(
        s.call(
            "calendar.import.apply",
            &json!({"content":content,"confirm":true})
        )
        .is_err()
    );
    let result = s
        .call(
            "calendar.import.apply",
            &json!({"content":content,"confirm":true,"allow_invalid":true}),
        )
        .unwrap();
    assert_eq!(result["created"].as_array().unwrap().len(), 1);
}
#[test]
fn malformed_and_ambiguous_timezone_events_are_rejected() {
    let mut s = db();
    let base = fixture()
        .replace(
            "DTSTART:20261110T090000",
            "DTSTART;TZID=America/New_York:20261101T013000",
        )
        .replace(
            "DTEND:20261110T100000",
            "DTEND;TZID=America/New_York:20261101T023000",
        );
    let preview = s
        .call("calendar.import.preview", &json!({"content":base}))
        .unwrap();
    assert_eq!(preview["invalid"].as_array().unwrap().len(), 1);
    assert_eq!(preview["valid"].as_array().unwrap().len(), 1);
    assert!(
        preview["invalid"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("Ambiguous DST")
    );
    assert!(
        s.call("calendar.import.preview", &json!({"content":"invalid"}))
            .is_err()
    );
}
