use crate::business::{Error, Result, require, text};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod calendar;
pub mod policy;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StudentRelationship {
    Enrolled,
    NewStudent,
}
impl StudentRelationship {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enrolled => "enrolled",
            Self::NewStudent => "new_student",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Submit {
    pub to_school_id: Uuid,
    pub student_relationship: StudentRelationship,
    pub declared_student_name: String,
    pub declared_student_number: Option<String>,
    pub reason: String,
    pub document_ids: Vec<Uuid>,
    pub consent_version: String,
    pub submission_key: Uuid,
}
impl Submit {
    pub fn validate(&mut self, policy: &policy::Policy) -> Result<()> {
        self.declared_student_name = text(&self.declared_student_name, 1, 100)?;
        self.reason = text(&self.reason, 1, 500)?;
        self.declared_student_number = self
            .declared_student_number
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| text(value, 1, 100))
            .transpose()?;
        require(
            !policy.student_number_required || self.declared_student_number.is_some(),
            "请填写学生编号",
        )?;
        require(
            self.consent_version == policy.version,
            "处理告知已更新，请重新确认",
        )?;
        require(
            !self.submission_key.is_nil() && !self.to_school_id.is_nil(),
            "申请标识无效",
        )?;
        self.document_ids.sort();
        require(
            !self.document_ids.is_empty() && self.document_ids.len() <= policy.max_documents,
            "证明材料数量不合法",
        )?;
        require(
            self.document_ids.windows(2).all(|pair| pair[0] != pair[1]),
            "材料不能重复选择",
        )
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub revision: i64,
    pub reason: String,
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filters {
    #[serde(rename = "page[number]")]
    pub page_number: Option<i64>,
    #[serde(rename = "page[size]")]
    pub page_size: Option<i64>,
    pub status: Option<String>,
    pub account_id: Option<Uuid>,
    pub created_from: Option<chrono::DateTime<chrono::Utc>>,
    pub created_to: Option<chrono::DateTime<chrono::Utc>>,
}
impl Filters {
    pub fn page(&self) -> crate::business::Page {
        crate::business::Page {
            page: self.page_number,
            page_size: self.page_size,
            ..Default::default()
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.page().fetch_bounds()?;
        if let Some(status) = &self.status {
            require(is_status(status), "申请状态无效")?;
        }
        require(
            self.created_from
                .zip(self.created_to)
                .is_none_or(|(from, to)| from <= to),
            "时间范围无效",
        )
    }
}
pub fn is_status(value: &str) -> bool {
    matches!(
        value,
        "pending" | "approved" | "rejected" | "cancelled" | "invalidated"
    )
}
pub fn pending(status: &str, actual: i64, expected: i64) -> Result<()> {
    if status != "pending" || actual != expected {
        return Err(Error::Policy("school_change_conflict"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_enrolled_students_and_new_students_can_be_declared() {
        for accepted in ["enrolled", "new_student"] {
            assert!(
                serde_json::from_value::<StudentRelationship>(serde_json::json!(accepted)).is_ok()
            );
        }
        for rejected in ["graduated", "suspended", "other", "", "joint_training"] {
            assert!(
                serde_json::from_value::<StudentRelationship>(serde_json::json!(rejected)).is_err()
            );
        }
    }
    #[test]
    fn terminals_cannot_be_reviewed_again() {
        for status in ["approved", "rejected", "cancelled", "invalidated"] {
            assert!(pending(status, 1, 1).is_err());
        }
        assert!(pending("pending", 2, 1).is_err());
        assert!(pending("pending", 2, 2).is_ok());
        assert!(!is_status("expired"));
    }
}
