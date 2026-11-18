use crate::{
    business::{Actor, Error, Result, require},
    statistics::{
        domain::*,
        south::{adapter::postgres::Repository, port::StatisticsStore},
    },
};
use uuid::Uuid;
#[derive(Clone)]
pub struct Service<R: StatisticsStore = Repository>(pub R, std::sync::Arc<tokio::sync::Semaphore>);
impl<R: StatisticsStore> Service<R> {
    pub fn new(store: R, concurrency: usize) -> Self {
        Self(
            store,
            std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency)),
        )
    }
    pub async fn validate_activity(&self, a: &Actor, v: &ActivityInput) -> Result<()> {
        self.0.validate_activity(a, v).await
    }
    pub async fn report(&self, a: &Actor, s: Option<Uuid>, q: Query) -> Result<Report> {
        if !a.is_admin_browser() {
            return Err(Error::Forbidden);
        }
        if let Some(s) = s {
            a.authorize(s, "operator")?;
        } else {
            a.platform()?;
        }
        let _permit = self.1.try_acquire().map_err(|_| Error::Unavailable)?;
        tokio::time::timeout(std::time::Duration::from_secs(60), self.0.report(a, s, q))
            .await
            .map_err(|_| Error::Unavailable)?
    }
    pub async fn snapshot(&self, a: &Actor, s: Option<Uuid>, id: Uuid) -> Result<Report> {
        if !a.is_admin_browser() {
            return Err(Error::Forbidden);
        }
        if let Some(s) = s {
            a.authorize(s, "operator")?;
        } else {
            a.platform()?;
        }
        self.0.snapshot(a, s, id).await
    }
}
pub fn csv(report: &Report, kind: &str) -> Result<Vec<u8>> {
    require(matches!(kind, "periods" | "schools"), "导出类型无效")?;
    require(
        kind != "schools" || report.school_id.is_none(),
        "单校不支持学校对比导出",
    )?;
    let mut writer = csv::Writer::from_writer(Vec::new());
    let header = [
        "范围",
        "周期开始",
        "周期结束",
        "时区",
        "数据截至",
        "口径版本",
        "覆盖状态",
        "新增帖子",
        "新增评论",
        "发帖人数",
        "评论人数",
        "内容参与人数",
        "新增注册",
        "期末累计注册",
        "新注册参与人数",
        "访问活跃人数",
        "注册人员内容参与人数",
        "日均发帖",
        "日均评论",
        "评论/发帖量比",
        "新注册参与率(%)",
        "注册人员参与率(%)",
    ];
    writer
        .write_record(header)
        .map_err(|_| Error::Unavailable)?;
    let safe = |s: &str| {
        if s.trim_start()
            .starts_with(['=', '+', '-', '@', '\t', '\r', '\n'])
        {
            format!("'{s}")
        } else {
            s.into()
        }
    };
    let mut row = |name: &str, w: &Window, m: &Metrics, c: &Coverage| -> Result<()> {
        let mut values = vec![
            safe(name),
            w.start_at
                .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
                .to_rfc3339(),
            w.actual_end_at
                .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
                .to_rfc3339(),
            report.timezone.clone(),
            report
                .as_of
                .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
                .to_rfc3339(),
            report.definition_version.to_string(),
            if w.state == "future" {
                "future".into()
            } else if c.reasons.is_empty() {
                w.state.clone()
            } else {
                c.reasons.join("；")
            },
        ];
        for n in [
            m.posts,
            m.comments,
            m.post_authors,
            m.comment_authors,
            m.participants,
            m.registrations,
            m.cumulative_registrations,
            m.new_participants,
            m.active_users,
            m.registered_participants,
        ] {
            values.push(n.map(|x| x.to_string()).unwrap_or_default());
        }
        for n in [
            m.daily_posts,
            m.daily_comments,
            m.comment_post_ratio,
            m.new_participation_rate,
            m.participation_rate,
        ] {
            values.push(n.map(|x| x.to_string()).unwrap_or_default());
        }
        writer.write_record(values).map_err(|_| Error::Unavailable)
    };
    if kind == "schools" {
        for school in &report.schools {
            row(
                &school.name,
                &report.window,
                &school.metrics,
                &school.coverage,
            )?;
        }
    } else {
        let name = if report.full_site {
            "全站".into()
        } else {
            report.school_names.join("、")
        };
        for b in &report.buckets {
            row(&name, &b.window, &b.metrics, &b.coverage)?;
        }
        row(
            &format!("{name} 合计（人数去重，累计取期末）"),
            &report.window,
            &report.summary,
            &report.coverage,
        )?;
    }
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend(writer.into_inner().map_err(|_| Error::Unavailable)?);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_uses_shanghai_time_and_escapes_formula_text() {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/platform-statistics.json"
        ))
        .unwrap();
        let mut attributes = doc["data"]["attributes"].clone();
        attributes["id"] = doc["data"]["id"].clone();
        let mut report: Report = serde_json::from_value(attributes).unwrap();
        report.schools[0].name = "=1+1".into();
        let bytes = csv(&report, "schools").unwrap();
        assert!(bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("'=1+1"));
        assert!(text.contains("2026-11-16T00:00:00+08:00"));
    }
}
