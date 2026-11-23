use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};
fn db() -> Store {
    Store::open(std::env::temp_dir().join(format!("life-attachment-{}.sqlite", id()))).unwrap()
}
#[test]
fn managed_attachments_are_durable_deduplicated_and_verified() {
    let mut s = db();
    let note = s
        .call("notes.create", &json!({"title":"Reference note"}))
        .unwrap();
    let input = json!({"owner_kind":"notes","owner_id":note["id"],"filename":"design-notes.txt","mime_type":"text/plain","data_base64":STANDARD.encode(b"FigCraft reference")});
    let attachment = s.call("attachments.create", &input).unwrap();
    assert_eq!(attachment["bytes"], 18);
    assert_eq!(attachment["sha256"].as_str().unwrap().len(), 64);
    assert!(attachment.get("data_base64").is_none());
    let same = s.call("attachments.create", &input).unwrap();
    assert_eq!(same["id"], attachment["id"]);
    let listed = s
        .call(
            "attachments.list",
            &json!({"owner_kind":"notes","owner_id":note["id"]}),
        )
        .unwrap();
    assert_eq!(listed.as_array().unwrap().len(), 1);
    let returned = s
        .call("attachments.get", &json!({"id":attachment["id"]}))
        .unwrap();
    assert_eq!(
        STANDARD
            .decode(returned["data_base64"].as_str().unwrap())
            .unwrap(),
        b"FigCraft reference"
    );
    s.call("attachments.delete", &json!({"id":attachment["id"]}))
        .unwrap();
    assert_eq!(
        s.call(
            "attachments.list",
            &json!({"owner_kind":"notes","owner_id":note["id"]})
        )
        .unwrap()
        .as_array()
        .unwrap()
        .len(),
        0
    );
    assert!(
        s.call("attachments.get", &json!({"id":attachment["id"]}))
            .is_err()
    );
    s.call("attachments.restore", &json!({"id":attachment["id"]}))
        .unwrap();
    assert_eq!(
        s.call(
            "attachments.list",
            &json!({"owner_kind":"notes","owner_id":note["id"]})
        )
        .unwrap()
        .as_array()
        .unwrap()
        .len(),
        1
    );
}
#[test]
fn attachments_reject_unsafe_filenames_invalid_ownership_and_large_payloads() {
    let mut s = db();
    let note = s
        .call("notes.create", &json!({"title":"Safe note"}))
        .unwrap();
    for name in ["../secret", "a/b.txt", "a\\b.txt", ""] {
        assert!(s.call("attachments.create",&json!({"owner_kind":"notes","owner_id":note["id"],"filename":name,"data_base64":STANDARD.encode(b"x")})).is_err());
    }
    assert!(s.call("attachments.create",&json!({"owner_kind":"notes","owner_id":"does-not-exist","filename":"x.txt","data_base64":STANDARD.encode(b"x")})).is_err());
    assert!(s.call("attachments.create",&json!({"owner_kind":"notes","owner_id":note["id"],"filename":"x.txt","data_base64":"###"})).is_err());
    assert!(s.call("attachments.create",&json!({"owner_kind":"notes","owner_id":note["id"],"filename":"x.txt","data_base64":STANDARD.encode(b"data"),"request_id":"do-not-log-payload"})).is_err());
}
