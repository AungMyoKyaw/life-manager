use super::*;
use chrono::{Datelike, NaiveDateTime, NaiveTime, Timelike};

fn clock(s: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M")
        .map_err(|_| LifeError::Invalid(format!("Invalid time {s}; expected HH:MM")))
}
fn timestamp(s: &str) -> Option<NaiveDateTime> {
    let local = s.get(..19).unwrap_or(s);
    NaiveDateTime::parse_from_str(local, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(local, "%Y-%m-%dT%H:%M"))
        .ok()
}
fn overlaps(a: (NaiveDateTime, NaiveDateTime), b: (NaiveDateTime, NaiveDateTime)) -> bool {
    a.0 < b.1 && a.1 > b.0
}
impl Store {
    pub fn plan_capacity(&self, p: &Value) -> Result<Value> {
        let date = optional(p, "date").unwrap_or_else(|| Local::now().date_naive().to_string());
        let day = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
            .map_err(|_| LifeError::Invalid("date must be YYYY-MM-DD".into()))?;
        let weekday = i64::from(day.weekday().num_days_from_monday());
        let rules = self.rows(
            "SELECT weekday,start_time,end_time,kind FROM availability_rules ORDER BY start_time",
            &[],
        )?;
        let has_rules = rules.as_array().is_some_and(|a| !a.is_empty());
        let mut available = Vec::new();
        let mut occupied: Vec<(NaiveDateTime, NaiveDateTime)> = Vec::new();
        if !has_rules && weekday < 5 {
            available.push((
                day.and_hms_opt(9, 0, 0).unwrap(),
                day.and_hms_opt(18, 0, 0).unwrap(),
            ));
        }
        for rule in rules.as_array().into_iter().flatten() {
            if rule["weekday"].as_i64() != Some(weekday) {
                continue;
            }
            let a = day.and_time(clock(rule["start_time"].as_str().unwrap_or(""))?);
            let b = day.and_time(clock(rule["end_time"].as_str().unwrap_or(""))?);
            if b <= a {
                return Err(LifeError::Invalid(
                    "Availability must end after start".into(),
                ));
            }
            if rule["kind"] == "available" {
                available.push((a, b));
            } else {
                occupied.push((a, b));
            }
        }
        available.sort_unstable();
        let mut merged: Vec<(NaiveDateTime, NaiveDateTime)> = Vec::new();
        for (a, b) in available {
            if let Some(last) = merged.last_mut()
                && a <= last.1
            {
                last.1 = last.1.max(b);
                continue;
            }
            merged.push((a, b));
        }
        let capacity_minutes: i64 = merged.iter().map(|(a, b)| (*b - *a).num_minutes()).sum();
        let events=self.rows("SELECT id,start_at,end_at,task_id FROM events WHERE substr(start_at,1,10)<=?1 AND substr(end_at,1,10)>=?1",&[&date])?;
        let mut booked = std::collections::HashSet::new();
        for event in events.as_array().into_iter().flatten() {
            if let (Some(a), Some(b)) = (
                event["start_at"].as_str().and_then(timestamp),
                event["end_at"].as_str().and_then(timestamp),
            ) && b > a
            {
                occupied.push((a, b));
            }
            if let Some(task_id) = event["task_id"].as_str() {
                booked.insert(task_id.to_string());
            }
        }
        // The part of today that has already passed cannot be scheduled. The
        // reference time can be supplied so the proposal is deterministic.
        let reference = match optional(p, "now") {
            Some(value) => NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M:%S")
                .or_else(|_| NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M"))
                .map_err(|_| LifeError::Invalid("now must be YYYY-MM-DDTHH:MM:SS".into()))?,
            None => Local::now().naive_local(),
        };
        if day == reference.date() {
            let next =
                reference + Duration::minutes(i64::from((5 - reference.minute() % 5) % 5) + 1);
            occupied.push((day.and_hms_opt(0, 0, 0).unwrap(), next));
        }
        let buffer = integer(p, "buffer_minutes", 10);
        if !(0..=60).contains(&buffer) {
            return Err(LifeError::Invalid(
                "buffer_minutes must be 0 through 60".into(),
            ));
        }
        let tasks=self.rows("SELECT id,title,priority,estimate_minutes,due_date FROM tasks WHERE deleted_at IS NULL AND status NOT IN ('done','cancelled') ORDER BY priority ASC, COALESCE(due_date,'9999-12-31') ASC LIMIT 100",&[])?;
        let mut slots = Vec::new();
        let mut unscheduled = Vec::new();
        let mut scheduled_minutes = 0;
        for task in tasks.as_array().into_iter().flatten() {
            let task_id = task["id"].as_str().unwrap_or("");
            if booked.contains(task_id) {
                continue;
            }
            let requested = task["estimate_minutes"].as_i64().unwrap_or(30).max(5);
            let minutes = requested.min(180);
            let mut chosen = None;
            'window: for (window_start, window_end) in &merged {
                let mut cursor = *window_start;
                while cursor + Duration::minutes(minutes) <= *window_end {
                    let interval = (cursor, cursor + Duration::minutes(minutes));
                    if let Some((_, end)) = occupied.iter().find(|span| overlaps(interval, **span))
                    {
                        cursor = (*end).max(cursor + Duration::minutes(5));
                        let remainder = cursor.minute() % 5;
                        if remainder > 0 {
                            cursor += Duration::minutes(i64::from(5 - remainder));
                        }
                        continue;
                    }
                    chosen = Some(interval);
                    break 'window;
                }
            }
            if let Some((a, b)) = chosen {
                slots.push(json!({"task_id":task_id,"title":task["title"],"start_at":a.format("%Y-%m-%dT%H:%M:%S").to_string(),"end_at":b.format("%Y-%m-%dT%H:%M:%S").to_string(),"minutes":minutes,"remaining_minutes":requested-minutes}));
                occupied.push((a, b + Duration::minutes(buffer)));
                scheduled_minutes += minutes;
            } else {
                unscheduled.push(json!({"task_id":task_id,"title":task["title"],"estimate_minutes":requested,"reason":"No conflict-free available interval"}));
            }
        }
        slots.sort_by_key(|s| s["start_at"].as_str().unwrap_or("").to_owned());
        Ok(
            json!({"date":date,"status":"proposal_only","slots":slots,"unscheduled":unscheduled,"capacity_minutes":capacity_minutes,"scheduled_minutes":scheduled_minutes,"buffer_minutes":buffer,"calendar_events":events.as_array().map_or(0,Vec::len),"warning":"Preview only. Local wall-clock intervals respect availability and stored events; external calendars and DST-aware provider sync are separate."}),
        )
    }
}
