use super::policy::Policy;
use crate::business::{Error, Result};
use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Utc, Weekday};

pub fn review_deadline(submitted: DateTime<Utc>, policy: &Policy) -> Result<DateTime<Utc>> {
    let zone = FixedOffset::east_opt(8 * 3600).ok_or(Error::Unavailable)?;
    let local = submitted.with_timezone(&zone);
    let mut date = local.date_naive();
    let mut remaining = policy.review_working_days;
    while remaining > 0 {
        date = date.succ_opt().ok_or(Error::Unavailable)?;
        let working = policy.review_workdays.contains(&date)
            || (!matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
                && !policy.review_holidays.contains(&date));
        if working {
            remaining -= 1;
        }
    }
    zone.from_local_datetime(&date.and_time(local.time()))
        .single()
        .map(|time| time.with_timezone(&Utc))
        .ok_or(Error::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn policy() -> Policy {
        serde_json::from_value(json!({
            "version":"test", "notice":"synthetic", "review_guidance":"synthetic",
            "private_root":"/tmp/synthetic-school-calendar", "digest_key":"synthetic-key-at-least-32-characters",
            "student_number_required":false, "restore_revoked_member":false,
            "max_file_bytes":1024,"max_total_bytes":2048,"max_pixels":100,
            "max_documents":1,"max_unbound_documents":2,"min_submission_seconds":1,
            "temporary_seconds":3600,"document_retention_seconds":3600,
            "personal_retention_seconds":3600,"audit_retention_seconds":7200,"cleanup_interval_seconds":60
        })).unwrap()
    }
    fn time(value: &str) -> DateTime<Utc> {
        value.parse().unwrap()
    }
    #[test]
    fn defaults_to_three_working_days_excluding_submission_date_and_weekends() {
        let policy = policy();
        assert_eq!(policy.review_working_days, 3);
        policy.validate().unwrap();
        assert_eq!(
            review_deadline(time("2026-11-20T04:30:00Z"), &policy).unwrap(),
            time("2026-11-25T04:30:00Z")
        );
        assert_eq!(
            review_deadline(time("2026-11-21T04:30:00Z"), &policy).unwrap(),
            time("2026-11-25T04:30:00Z")
        );
    }
    #[test]
    fn uses_beijing_date_even_when_utc_date_is_previous_day() {
        assert_eq!(
            review_deadline(time("2026-11-22T16:30:00Z"), &policy()).unwrap(),
            time("2026-11-25T16:30:00Z")
        );
    }
    #[test]
    fn supports_explicit_holidays_and_makeup_workdays() {
        let mut policy = policy();
        policy.review_holidays.push("2026-11-23".parse().unwrap());
        assert_eq!(
            review_deadline(time("2026-11-20T04:30:00Z"), &policy).unwrap(),
            time("2026-11-26T04:30:00Z")
        );
        policy.review_holidays.clear();
        policy.review_workdays.push("2026-11-21".parse().unwrap());
        assert_eq!(
            review_deadline(time("2026-11-20T04:30:00Z"), &policy).unwrap(),
            time("2026-11-24T04:30:00Z")
        );
    }
    #[test]
    fn rejects_conflicting_calendar_dates_and_zero_day_deadlines() {
        let mut policy = policy();
        policy.review_working_days = 0;
        assert!(policy.validate().is_err());
        policy.review_working_days = 3;
        policy.review_holidays.push("2026-11-23".parse().unwrap());
        policy.review_workdays = policy.review_holidays.clone();
        assert!(policy.validate().is_err());
    }
}
