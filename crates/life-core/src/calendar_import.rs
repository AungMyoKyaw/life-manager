//! Deliberate one-way RFC5545 VEVENT import. Unsupported recurrence and
//! ambiguous timezone transitions are reported, not silently invented.
use super::*;
use chrono::{LocalResult, TimeZone};
use std::collections::HashSet;

fn unescape_ical(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n' | 'N') => out.push('\n'),
                Some('\\') => out.push('\\'),
                Some(',') => out.push(','),
                Some(';') => out.push(';'),
                Some(other) => {
                    out.push('\\');
                    out.push(other)
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}
fn unfolded(content: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    for row in content
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .split('\n')
    {
        if let Some(first) = row.chars().next()
            && (first == ' ' || first == '\t')
            && let Some(last) = result.last_mut()
        {
            last.push_str(&row[1..]);
            continue;
        }
        result.push(row.to_owned());
    }
    result
}
fn property<'a>(lines: &'a [String], field: &str) -> Option<(&'a str, &'a str)> {
    lines.iter().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name
            .split(';')
            .next()
            .is_some_and(|s| s.eq_ignore_ascii_case(field))
        {
            Some((name, value))
        } else {
            None
        }
    })
}
fn parse_date(name: &str, value: &str) -> Result<(String, bool, String)> {
    let all_day = name
        .split(';')
        .any(|part| part.eq_ignore_ascii_case("VALUE=DATE"))
        || value.len() == 8;
    if all_day {
        let day = NaiveDate::parse_from_str(value, "%Y%m%d")
            .map_err(|_| LifeError::Invalid("Invalid ICS DATE format".into()))?;
        return Ok((
            day.and_hms_opt(0, 0, 0)
                .unwrap()
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
            true,
            "floating-date".into(),
        ));
    }
    let utc = value.ends_with('Z');
    let raw = if utc {
        value.strip_suffix('Z').unwrap_or(value)
    } else {
        value
    };
    let naive = chrono::NaiveDateTime::parse_from_str(raw, "%Y%m%dT%H%M%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(raw, "%Y%m%dT%H%M"))
        .map_err(|_| LifeError::Invalid("Invalid ICS DATE-TIME".into()))?;
    let zone = name.split(';').find_map(|part| part.strip_prefix("TZID="));
    let (local, timezone) = if utc {
        let date = chrono::DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);
        (date.with_timezone(&Local).naive_local(), "UTC".to_string())
    } else if let Some(zone) = zone {
        let tz: chrono_tz::Tz = zone
            .parse()
            .map_err(|_| LifeError::Invalid(format!("Unknown ICS timezone {zone}")))?;
        let value = match tz.from_local_datetime(&naive) {
            LocalResult::Single(v) => v,
            LocalResult::Ambiguous(_, _) => {
                return Err(LifeError::Invalid(format!(
                    "Ambiguous DST hour in {zone}; manual review required"
                )));
            }
            LocalResult::None => {
                return Err(LifeError::Invalid(format!(
                    "Nonexistent DST hour in {zone}; manual review required"
                )));
            }
        };
        (value.with_timezone(&Local).naive_local(), zone.to_owned())
    } else {
        (naive, "floating-local".into())
    };
    Ok((
        local.format("%Y-%m-%dT%H:%M:%S").to_string(),
        false,
        timezone,
    ))
}
fn parse_event(lines: &[String]) -> Result<Value> {
    for unsupported in ["RRULE", "RDATE", "EXDATE", "RECURRENCE-ID"] {
        if property(lines, unsupported).is_some() {
            return Err(LifeError::Invalid(format!(
                "{unsupported} import is not supported without a recurrence expansion engine"
            )));
        }
    }
    let uid = property(lines, "UID")
        .map(|(_, v)| v.trim())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| LifeError::Invalid("VEVENT requires UID".into()))?;
    if uid.len() > 200 || uid.chars().any(char::is_control) {
        return Err(LifeError::Invalid("Invalid UID".into()));
    }
    let title = property(lines, "SUMMARY")
        .map(|(_, v)| unescape_ical(v))
        .unwrap_or_default();
    if title.trim().is_empty() || title.chars().count() > 250 {
        return Err(LifeError::Invalid(
            "VEVENT requires a title of 1–250 characters".into(),
        ));
    }
    let start_prop = property(lines, "DTSTART")
        .ok_or_else(|| LifeError::Invalid("VEVENT missing DTSTART".into()))?;
    let end_prop = property(lines, "DTEND")
        .ok_or_else(|| LifeError::Invalid("VEVENT missing DTEND".into()))?;
    let (start, all_day, timezone) = parse_date(start_prop.0, start_prop.1)?;
    let (end, end_all_day, _) = parse_date(end_prop.0, end_prop.1)?;
    if all_day != end_all_day {
        return Err(LifeError::Invalid(
            "Mixed DATE and DATE-TIME event boundaries".into(),
        ));
    }
    if end <= start {
        return Err(LifeError::Invalid(
            "VEVENT ends before or at its start".into(),
        ));
    }
    let description = property(lines, "DESCRIPTION")
        .map(|(_, v)| unescape_ical(v))
        .unwrap_or_default();
    if description.chars().count() > 20000 {
        return Err(LifeError::Invalid("Event description is too long".into()));
    }
    let location = property(lines, "LOCATION")
        .map(|(_, v)| unescape_ical(v))
        .unwrap_or_default();
    Ok(
        json!({"uid":uid,"title":title,"description":description,"location":location,"start_at":start,"end_at":end,"all_day":all_day,"timezone_id":timezone}),
    )
}
fn parse_calendar(content: &str) -> Result<Vec<std::result::Result<Value, String>>> {
    if content.is_empty() || content.len() > 4 * 1024 * 1024 {
        return Err(LifeError::Invalid(
            "ICS must be between 1 byte and 4 MiB".into(),
        ));
    }
    if !content.contains("BEGIN:VCALENDAR") {
        return Err(LifeError::Invalid("Not a VCALENDAR".into()));
    }
    let mut events = Vec::new();
    let mut current = Vec::new();
    let mut inside = false;
    for line in unfolded(content) {
        if line.eq_ignore_ascii_case("BEGIN:VEVENT") {
            inside = true;
            current.clear();
            continue;
        }
        if line.eq_ignore_ascii_case("END:VEVENT") {
            if inside {
                events.push(parse_event(&current).map_err(|e| e.to_string()));
                inside = false;
            }
            if events.len() > 2000 {
                return Err(LifeError::Invalid(
                    "ICS import limited to 2000 events".into(),
                ));
            }
            continue;
        }
        if inside {
            current.push(line)
        }
    }
    if inside {
        return Err(LifeError::Invalid("Unterminated VEVENT".into()));
    }
    if events.is_empty() {
        return Err(LifeError::Invalid("No VEVENT entries found".into()));
    }
    Ok(events)
}
impl Store {
    pub fn ics_preview(&self, p: &Value) -> Result<Value> {
        let content = require(p, "content")?;
        if p.get("request_id").is_some() {
            return Err(LifeError::Invalid(
                "ICS payloads may not use request_id; duplicate UIDs are skipped instead".into(),
            ));
        }
        let source = optional(p, "source").unwrap_or_else(|| "imported-calendar".into());
        if source.len() > 100 || source.chars().any(char::is_control) {
            return Err(LifeError::Invalid("Invalid calendar source label".into()));
        }
        let mut valid = Vec::new();
        let mut invalid = Vec::new();
        let mut duplicates = Vec::new();
        let mut seen = HashSet::new();
        for (index, record) in parse_calendar(&content)?.into_iter().enumerate() {
            match record {
                Err(reason) => invalid.push(json!({"index":index,"reason":reason})),
                Ok(event) => {
                    let uid = event["uid"].as_str().unwrap_or("");
                    let count: i64 = self.conn.query_row(
                        "SELECT COUNT(*) FROM events WHERE import_source=?1 AND external_uid=?2",
                        params![source, uid],
                        |r| r.get(0),
                    )?;
                    if !seen.insert(uid.to_owned()) || count > 0 {
                        duplicates.push(json!({"index":index,"uid":uid,"title":event["title"],"reason":"UID already imported or duplicated in file"}));
                    } else {
                        let mut item = event;
                        item["index"] = json!(index);
                        valid.push(item);
                    }
                }
            }
        }
        Ok(
            json!({"source":source,"total":valid.len()+invalid.len()+duplicates.len(),"valid":valid,"invalid":invalid,"duplicates":duplicates,"can_apply":true,"policy":"Explicit confirmation required; duplicates skipped; unsupported recurrence and ambiguous time zones classified as invalid"}),
        )
    }
    pub fn ics_apply(&self, p: &Value, actor: &str) -> Result<Value> {
        if p.get("confirm").and_then(Value::as_bool) != Some(true) {
            return Err(LifeError::Invalid(
                "confirm=true required after ICS preview".into(),
            ));
        }
        if p.get("request_id").is_some() {
            return Err(LifeError::Invalid("ICS content cannot be stored in idempotency payload logs; duplicate UIDs are automatically skipped".into()));
        }
        let preview = self.ics_preview(p)?;
        let invalid = preview["invalid"].as_array().cloned().unwrap_or_default();
        if !invalid.is_empty() && p.get("allow_invalid").and_then(Value::as_bool) != Some(true) {
            return Err(LifeError::Invalid("ICS contains unsupported or invalid events; review preview and set allow_invalid=true to skip".into()));
        }
        let source = preview["source"].as_str().unwrap_or("");
        let mut created = Vec::new();
        for event in preview["valid"].as_array().into_iter().flatten() {
            let key = id();
            self.conn.execute("INSERT INTO events(id,title,description,start_at,end_at,all_day,kind,color,locked,created_at,updated_at,external_uid,import_source,timezone_id) VALUES(?1,?2,?3,?4,?5,?6,'external','#8C9E90',1,?7,?7,?8,?9,?10)",params![
                key,event["title"].as_str(),event["description"].as_str(),event["start_at"].as_str(),
                event["end_at"].as_str(),i64::from(event["all_day"].as_bool().unwrap_or(false)),
                now(),event["uid"].as_str(),source,event["timezone_id"].as_str()
            ])?;
            created.push(json!({"id":key,"uid":event["uid"],"title":event["title"]}));
        }
        self.record(actor,"calendar.import.apply",None,&json!({"source":source,"created":created.len(),"skipped_invalid":invalid.len(),"skipped_duplicates":preview["duplicates"].as_array().map_or(0,Vec::len)}))?;
        Ok(
            json!({"created":created,"skipped_invalid":invalid,"skipped_duplicates":preview["duplicates"],"source":source,"atomic":true}),
        )
    }
}
