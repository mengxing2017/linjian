use crate::business::{Error, Result, require};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub version: String,
    pub notice: String,
    pub review_guidance: String,
    pub private_root: PathBuf,
    pub digest_key: String,
    pub student_number_required: bool,
    pub restore_revoked_member: bool,
    pub max_file_bytes: usize,
    pub max_total_bytes: i64,
    pub max_pixels: u64,
    pub max_documents: usize,
    pub max_unbound_documents: i64,
    pub min_submission_seconds: i64,
    pub temporary_seconds: i64,
    #[serde(default = "default_review_working_days")]
    pub review_working_days: u32,
    #[serde(default)]
    pub review_holidays: Vec<chrono::NaiveDate>,
    #[serde(default)]
    pub review_workdays: Vec<chrono::NaiveDate>,
    pub document_retention_seconds: i64,
    pub personal_retention_seconds: i64,
    pub audit_retention_seconds: i64,
    pub cleanup_interval_seconds: u64,
}
impl Policy {
    pub fn defaults(private_root: PathBuf, digest_key: String) -> Self {
        Self {
            version: "school-change-v1".into(),
            notice: "仅接受目标学校在读或新生申请。材料仅用于本次审核，平台审核人员可访问；审核通过后立即停止访问并自动删除，其他结案材料保留24小时，未提交材料保留24小时。声明信息保留30天，操作审计保留90天。默认3个工作日未审核自动驳回，可撤回待审申请或重新申请；如有异议请通过平台反馈联系处理。".into(),
            review_guidance: "核验目标学校、材料所属本人及有效的在读或新生关系；不接受其他学生关系。新生可使用录取或入学证明；无学生编号可不填写。批准前确认材料与声明一致。".into(),
            private_root,
            digest_key,
            student_number_required: false,
            restore_revoked_member: false,
            max_file_bytes: 5 * 1024 * 1024,
            max_total_bytes: 15 * 1024 * 1024,
            max_pixels: 24_000_000,
            max_documents: 3,
            max_unbound_documents: 10,
            min_submission_seconds: 1,
            temporary_seconds: 86400,
            review_working_days: 3,
            review_holidays: vec![],
            review_workdays: vec![],
            document_retention_seconds: 86400,
            personal_retention_seconds: 30 * 86400,
            audit_retention_seconds: 90 * 86400,
            cleanup_interval_seconds: 60,
        }
    }
    pub fn validate(&self) -> Result<()> {
        require(
            (1..=365).contains(&self.review_working_days)
                && self.review_holidays.len() <= 3660
                && self.review_workdays.len() <= 3660
                && !self
                    .review_holidays
                    .iter()
                    .any(|date| self.review_workdays.contains(date)),
            "审核工作日或节假日日历配置无效",
        )?;
        require(
            !self.version.trim().is_empty()
                && !self.notice.trim().is_empty()
                && !self.review_guidance.trim().is_empty(),
            "换校告知及审核规则未配置",
        )?;
        require(
            self.private_root.is_absolute() && self.digest_key.len() >= 32,
            "私有目录或幂等摘要密钥无效",
        )?;
        require(
            (1..=5 * 1024 * 1024).contains(&self.max_file_bytes)
                && (1..=24_000_000).contains(&self.max_pixels),
            "材料大小/像素限制无效",
        )?;
        require(
            (1..=10).contains(&self.max_documents)
                && self.max_unbound_documents >= self.max_documents as i64
                && self.max_unbound_documents <= 100,
            "材料配额无效",
        )?;
        require(
            self.max_total_bytes >= self.max_file_bytes as i64
                && self.max_total_bytes <= 50 * 1024 * 1024,
            "材料总大小无效",
        )?;
        for seconds in [
            self.min_submission_seconds,
            self.temporary_seconds,
            self.document_retention_seconds,
            self.personal_retention_seconds,
            self.audit_retention_seconds,
        ] {
            require((1..=315_360_000).contains(&seconds), "换校期限配置无效")?;
        }
        require(
            self.audit_retention_seconds
                >= self
                    .document_retention_seconds
                    .max(self.personal_retention_seconds),
            "审计保留期限不能短于审核材料与声明字段期限",
        )
        .and_then(|()| {
            require(
                (1..=3600).contains(&self.cleanup_interval_seconds),
                "清理周期必须在 1 至 3600 秒之间",
            )
        })
    }
    pub fn from_env() -> Result<Option<Self>> {
        let path = match std::env::var("LINJIAN_SCHOOL_CHANGE_POLICY_FILE") {
            Ok(path) if !path.trim().is_empty() => path,
            Ok(_) | Err(std::env::VarError::NotPresent) => return Ok(None),
            Err(_) => return Err(Error::Invalid("换校策略文件路径无效".into())),
        };
        let bytes = std::fs::read(path).map_err(|_| Error::Unavailable)?;
        let policy: Self = serde_json::from_slice(&bytes)
            .map_err(|_| Error::Invalid("换校策略文件无效".into()))?;
        policy.validate()?;
        Ok(Some(policy))
    }
}

fn default_review_working_days() -> u32 {
    3
}
