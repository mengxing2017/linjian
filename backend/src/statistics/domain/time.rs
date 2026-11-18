use super::{Query, Window};
use crate::business::{Error, Result, require};
use chrono::{
    DateTime, Datelike, Duration, FixedOffset, NaiveDate, TimeZone, Timelike, Utc, Weekday,
};
#[derive(Clone)]
pub struct Plan {
    pub period: String,
    pub granularity: String,
    pub window: Window,
    pub buckets: Vec<Window>,
    pub comparisons: Vec<(String, Option<Window>)>,
}
fn invalid() -> Error {
    Error::Invalid("统计日期或粒度无效".into())
}
fn midnight(d: NaiveDate) -> DateTime<Utc> {
    FixedOffset::east_opt(8 * 3600)
        .unwrap()
        .from_local_datetime(&d.and_hms_opt(0, 0, 0).unwrap())
        .unwrap()
        .with_timezone(&Utc)
}
fn month(d: NaiveDate, delta: i32) -> Option<NaiveDate> {
    let n = d.year() * 12 + d.month0() as i32 + delta;
    NaiveDate::from_ymd_opt(n.div_euclid(12), n.rem_euclid(12) as u32 + 1, d.day())
}
fn parse(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| invalid())
}
fn next(d: NaiveDate, g: &str) -> Result<NaiveDate> {
    match g {
        "day" => d.succ_opt(),
        "week" => d.checked_add_signed(Duration::days(7)),
        "month" => month(d, 1),
        _ => None,
    }
    .ok_or_else(invalid)
}
fn win(key: &str, start: DateTime<Utc>, end: DateTime<Utc>, now: DateTime<Utc>) -> Window {
    Window {
        key: key.into(),
        start_at: start,
        end_at: end,
        actual_end_at: end.min(now).max(start),
        state: if start >= now {
            "future"
        } else if end > now {
            "in_progress"
        } else {
            "complete"
        }
        .into(),
    }
}
pub fn buckets(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    g: &str,
    now: DateTime<Utc>,
) -> Result<Vec<Window>> {
    let local = start.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
    let mut at = if g == "hour" {
        start
    } else {
        let d = local.date_naive();
        midnight(match g {
            "week" => d - Duration::days(d.weekday().num_days_from_monday() as i64),
            "month" => d.with_day(1).ok_or_else(invalid)?,
            _ => d,
        })
    };
    let mut out = Vec::new();
    while at < end {
        require(out.len() < 400, "最多允许 400 个统计桶")?;
        let stop = if g == "hour" {
            at + Duration::hours(1)
        } else {
            midnight(next(
                at.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
                    .date_naive(),
                g,
            )?)
        };
        let mut w = win(
            &format!("bucket-{}", out.len()),
            at.max(start),
            stop.min(end),
            now,
        );
        if at < start || stop > end {
            w.state = if w.state == "future" {
                "future"
            } else {
                "partial"
            }
            .into();
        }
        out.push(w);
        at = stop;
    }
    Ok(out)
}
impl Plan {
    pub fn new(q: &Query, now: DateTime<Utc>) -> Result<Self> {
        q.view()?;
        let today = now
            .with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
            .date_naive();
        let p = q.period.as_deref().unwrap_or("month");
        require(
            matches!(p, "day" | "week" | "month" | "year" | "custom"),
            "报表周期无效",
        )?;
        let anchor = q
            .anchor_date
            .as_deref()
            .map(parse)
            .transpose()?
            .unwrap_or(today);
        let (start, end) = if p == "custom" {
            require(q.anchor_date.is_none(), "自定义范围不能指定周期日期")?;
            let a = parse(q.start_date.as_deref().ok_or_else(invalid)?)?;
            let b = parse(q.end_date.as_deref().ok_or_else(invalid)?)?;
            require(
                a <= b && b <= today && (b - a).num_days() < 366,
                "自定义范围最多 366 天且不能包含未来日期",
            )?;
            (a, b.succ_opt().ok_or_else(invalid)?)
        } else {
            require(
                q.start_date.is_none() && q.end_date.is_none(),
                "自然周期不接受自定义日期",
            )?;
            require(anchor <= today, "不能选择未来周期")?;
            let a = match p {
                "week" => anchor - Duration::days(anchor.weekday().num_days_from_monday() as i64),
                "month" => anchor.with_day(1).ok_or_else(invalid)?,
                "year" => NaiveDate::from_ymd_opt(anchor.year(), 1, 1).ok_or_else(invalid)?,
                _ => anchor,
            };
            let b = match p {
                "year" => NaiveDate::from_ymd_opt(a.year() + 1, 1, 1),
                "month" => month(a, 1),
                "week" => a.checked_add_signed(Duration::days(7)),
                _ => a.succ_opt(),
            }
            .ok_or_else(invalid)?;
            (a, b)
        };
        let default = match p {
            "day" => "hour",
            "year" => "month",
            _ => "day",
        };
        let g = q.granularity.as_deref().unwrap_or(default);
        require(
            if p == "custom" {
                matches!(g, "day" | "week" | "month")
            } else {
                g == default
            },
            "粒度不适用于所选报表",
        )?;
        let w = win("summary", midnight(start), midnight(end), now);
        let bs = buckets(w.start_at, w.end_at, g, now)?;
        let selection = q.comparison.as_deref().unwrap_or("none");
        require(
            matches!(selection, "none" | "previous" | "year" | "both"),
            "对比类型无效",
        )?;
        let mut comparisons = Vec::new();
        for kind in ["previous", "year"] {
            if selection == "none"
                || selection != "both" && selection != kind
                || p == "custom" && kind == "year"
                || p == "year" && kind == "previous"
            {
                continue;
            }
            let shift = |d: NaiveDate| -> Option<NaiveDate> {
                match (kind, p) {
                    ("previous", "day") => d.pred_opt(),
                    ("previous", "week") => d.checked_sub_signed(Duration::days(7)),
                    ("previous", "month") => month(d, -1),
                    ("previous", "custom") => d.checked_sub_signed(end - start),
                    (_, "week") => NaiveDate::from_isoywd_opt(
                        d.iso_week().year() - 1,
                        d.iso_week().week(),
                        Weekday::Mon,
                    ),
                    _ => NaiveDate::from_ymd_opt(d.year() - 1, d.month(), d.day()),
                }
            };
            let mapped = shift(start).and_then(|a| {
                let b = if w.end_at > now {
                    let local = now.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
                    let d = if p == "week" {
                        a.checked_add_signed(local.date_naive() - start)
                    } else {
                        shift(local.date_naive())
                    }?;
                    let t = d.and_hms_nano_opt(
                        local.hour(),
                        local.minute(),
                        local.second(),
                        local.nanosecond(),
                    )?;
                    FixedOffset::east_opt(8 * 3600)?
                        .from_local_datetime(&t)
                        .single()?
                        .with_timezone(&Utc)
                } else {
                    let b = match p {
                        "month" => month(a, 1),
                        "year" => NaiveDate::from_ymd_opt(a.year() + 1, 1, 1),
                        "week" => a.checked_add_signed(Duration::days(7)),
                        "custom" => a.checked_add_signed(end - start),
                        _ => a.succ_opt(),
                    }?;
                    midnight(b)
                };
                Some(win(kind, midnight(a), b, now))
            });
            comparisons.push((kind.into(), mapped));
        }
        Ok(Self {
            period: p.into(),
            granularity: g.into(),
            window: w,
            buckets: bs,
            comparisons,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn now(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }
    #[test]
    fn boundaries_and_in_progress() {
        let q = Query {
            period: Some("month".into()),
            comparison: Some("both".into()),
            ..Default::default()
        };
        let p = Plan::new(&q, now("2026-11-18T07:30:00Z")).unwrap();
        assert_eq!(p.window.start_at, now("2026-10-31T16:00:00Z"));
        assert_eq!(p.buckets.len(), 30);
        assert_eq!(p.buckets[18].state, "future");
        assert_eq!(
            p.comparisons[0].1.as_ref().unwrap().end_at,
            now("2026-10-18T07:30:00Z")
        );
    }
    #[test]
    fn invalid_month_day_is_not_clamped() {
        let p = Plan::new(
            &Query {
                period: Some("month".into()),
                comparison: Some("both".into()),
                ..Default::default()
            },
            now("2026-03-31T07:00:00Z"),
        )
        .unwrap();
        assert!(p.comparisons[0].1.is_none());
    }
    #[test]
    fn leap_and_iso_week() {
        let p = Plan::new(
            &Query {
                period: Some("day".into()),
                comparison: Some("both".into()),
                anchor_date: Some("2024-02-29".into()),
                ..Default::default()
            },
            now("2026-01-01T00:00:00Z"),
        )
        .unwrap();
        assert!(p.comparisons[1].1.is_none());
        let p = Plan::new(
            &Query {
                period: Some("week".into()),
                comparison: Some("both".into()),
                anchor_date: Some("2020-12-31".into()),
                ..Default::default()
            },
            now("2026-01-01T00:00:00Z"),
        )
        .unwrap();
        assert!(p.comparisons[1].1.is_none());
    }
}
