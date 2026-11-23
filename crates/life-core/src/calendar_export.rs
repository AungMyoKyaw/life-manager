use super::*;

fn local_ical_datetime(s: &str, all_day: bool) -> Result<String> {
    let raw = s.get(..19).unwrap_or(s);
    let naive = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M"))
        .map_err(|_| LifeError::Invalid("Event has an invalid local datetime".into()))?;
    Ok(if all_day {
        naive.date().format("%Y%m%d").to_string()
    } else {
        naive.format("%Y%m%dT%H%M%S").to_string()
    })
}
fn ical_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace(';', "\\;")
        .replace(',', "\\,")
}
impl Store {
    pub fn calendar_export_ics(&self, p: &Value) -> Result<Value> {
        let events = self.events_list(p)?;
        let stamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let mut lines = vec![
            "BEGIN:VCALENDAR".to_string(),
            "VERSION:2.0".into(),
            "PRODID:-//Life Manager//Local Calendar//EN".into(),
            "CALSCALE:GREGORIAN".into(),
        ];
        let mut count = 0;
        for event in events.as_array().into_iter().flatten() {
            let key = event["id"]
                .as_str()
                .ok_or_else(|| LifeError::Invalid("Event row is missing ID".into()))?;
            let all_day = event["all_day"].as_i64() == Some(1);
            let start = local_ical_datetime(event["start_at"].as_str().unwrap_or(""), all_day)?;
            let end = local_ical_datetime(event["end_at"].as_str().unwrap_or(""), all_day)?;
            let uid = event["external_uid"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{key}@life-manager.local"));
            if uid.chars().any(char::is_control) {
                return Err(LifeError::Invalid("Event contains invalid UID".into()));
            }
            lines.extend([
                "BEGIN:VEVENT".into(),
                format!("UID:{}", ical_escape(&uid)),
                format!("DTSTAMP:{stamp}"),
                format!(
                    "DTSTART{}:{start}",
                    if all_day { ";VALUE=DATE" } else { "" }
                ),
                format!("DTEND{}:{end}", if all_day { ";VALUE=DATE" } else { "" }),
                format!(
                    "SUMMARY:{}",
                    ical_escape(event["title"].as_str().unwrap_or(""))
                ),
                format!(
                    "DESCRIPTION:{}",
                    ical_escape(event["description"].as_str().unwrap_or(""))
                ),
                "END:VEVENT".into(),
            ]);
            count += 1;
        }
        lines.push("END:VCALENDAR".into());
        Ok(json!({
            "content":lines.join("\r\n")+"\r\n","mime":"text/calendar","events":count,
            "timezone_note":"Local wall-clock date-time export. Original TZID is stored as metadata, but exported datetime values are floating local clock time; not live CalDAV sync."
        }))
    }
}
