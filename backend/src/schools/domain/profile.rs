use crate::business::{Error, Result, require, text};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchoolView {
    pub id: Uuid,
    pub name: String,
    pub short_name: String,
    pub province: String,
    pub city: String,
    pub area: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub status: String,
    pub logo_asset_id: Option<Uuid>,
    pub header_bg_asset_id: Option<Uuid>,
    pub logo_url: Option<String>,
    pub header_bg_image_url: Option<String>,
    pub theme_primary: String,
    pub theme_secondary: String,
    pub theme_text_color: String,
    pub revision: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub config: Value,
}
impl SchoolView {
    pub fn directory(&self) -> Value {
        json!({"id":self.id,"name":self.name,"short_name":self.short_name,"province":self.province,"city":self.city,"area":self.area,"latitude":self.latitude,"longitude":self.longitude,"status":self.status,"logo_url":self.logo_url,"header_bg_image_url":self.header_bg_image_url,"theme_primary":self.theme_primary,"theme_secondary":self.theme_secondary,"theme_text_color":self.theme_text_color,"revision":self.revision})
    }
    pub fn input(&self) -> SchoolInput {
        SchoolInput {
            name: self.name.clone(),
            short_name: self.short_name.clone(),
            province: self.province.clone(),
            city: self.city.clone(),
            area: self.area.clone(),
            latitude: self.latitude,
            longitude: self.longitude,
            status: Some(self.status.clone()),
            logo_asset_id: self.logo_asset_id,
            header_bg_asset_id: self.header_bg_asset_id,
            theme_primary: self.theme_primary.clone(),
            theme_secondary: self.theme_secondary.clone(),
            theme_text_color: self.theme_text_color.clone(),
            revision: self.revision,
            config: Some(self.config.clone()),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SchoolInput {
    pub name: String,
    pub short_name: String,
    pub province: String,
    pub city: String,
    pub area: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub status: Option<String>,
    pub logo_asset_id: Option<Uuid>,
    pub header_bg_asset_id: Option<Uuid>,
    pub theme_primary: String,
    pub theme_secondary: String,
    pub theme_text_color: String,
    pub revision: i64,
    pub config: Option<Value>,
}
impl Default for SchoolInput {
    fn default() -> Self {
        Self {
            name: String::new(),
            short_name: String::new(),
            province: String::new(),
            city: String::new(),
            area: String::new(),
            latitude: None,
            longitude: None,
            status: None,
            logo_asset_id: None,
            header_bg_asset_id: None,
            theme_primary: "#6A5AF9".into(),
            theme_secondary: "#8F6BFF".into(),
            theme_text_color: "#FFFFFF".into(),
            revision: 0,
            config: None,
        }
    }
}
pub fn color(v: &str) -> Result<String> {
    let v = v.trim();
    require(
        v.starts_with('#')
            && matches!(v.len(), 4 | 7)
            && v[1..].bytes().all(|c| c.is_ascii_hexdigit()),
        "颜色须为 #RGB 或 #RRGGBB",
    )?;
    Ok(if v.len() == 4 {
        format!(
            "#{}",
            v[1..].chars().flat_map(|c| [c, c]).collect::<String>()
        )
    } else {
        v.into()
    }
    .to_uppercase())
}
impl SchoolInput {
    pub fn validate(&mut self) -> Result<()> {
        self.name = text(&self.name, 1, 100)?;
        self.short_name = text(&self.short_name, 0, 50)?;
        self.province = text(&self.province, 0, 100)?;
        self.city = text(&self.city, 0, 100)?;
        self.area = text(&self.area, 0, 100)?;
        require(
            self.latitude.is_some() == self.longitude.is_some(),
            "经纬度须同时填写或清空",
        )?;
        for (v, max) in [(self.latitude, 90.0), (self.longitude, 180.0)] {
            if let Some(v) = v {
                require(
                    v.is_finite()
                        && v.abs() <= max
                        && (v * 1e6 - (v * 1e6).round()).abs() < 0.00001,
                    "坐标范围无效或超过六位小数",
                )?;
            }
        }
        require(
            self.status
                .as_deref()
                .is_none_or(|v| matches!(v, "normal" | "hidden")),
            "学校状态无效",
        )?;
        self.theme_primary = color(&self.theme_primary)?;
        self.theme_secondary = color(&self.theme_secondary)?;
        self.theme_text_color = color(&self.theme_text_color)?;
        if let Some(c) = &self.config {
            require(c.is_object() && c.to_string().len() < 16384, "学校配置无效")?;
            require(c.get("theme_color").is_none(), "请使用独立学校主题字段")?;
            for k in ["post_audit_mode", "comment_audit_mode"] {
                if let Some(v) = c.get(k) {
                    require(v == "manual" || v == "auto", "审核模式只能为 auto/manual")?;
                }
            }
        }
        Ok(())
    }
    pub fn parse(v: Value) -> Result<Self> {
        serde_json::from_value(v).map_err(|_| Error::Invalid("学校字段无效或包含未知字段".into()))
    }
}
#[derive(Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchoolQuery {
    #[serde(rename = "page[number]")]
    pub page: Option<i64>,
    #[serde(rename = "page[size]")]
    pub size: Option<i64>,
    pub search: Option<String>,
    pub province: Option<String>,
    pub city: Option<String>,
    pub area: Option<String>,
    pub status: Option<String>,
}
impl SchoolQuery {
    pub fn bounds(&self) -> Result<(i64, i64)> {
        let p = self.page.unwrap_or(1);
        let n = self.size.unwrap_or(30);
        require(
            (1..=100000).contains(&p) && (1..=100).contains(&n),
            "分页参数无效",
        )?;
        require(
            self.status
                .as_deref()
                .is_none_or(|v| matches!(v, "normal" | "hidden")),
            "学校状态无效",
        )?;
        for v in [&self.search, &self.province, &self.city, &self.area]
            .into_iter()
            .flatten()
        {
            text(v, 0, 100)?;
        }
        Ok((n, (p - 1) * n))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coordinates_colors_and_unknown_fields() {
        let mut v = SchoolInput {
            name: "校".into(),
            ..Default::default()
        };
        v.latitude = Some(0.0);
        assert!(v.validate().is_err());
        v.longitude = Some(0.0);
        v.theme_primary = "#aBc".into();
        v.validate().unwrap();
        assert_eq!(v.theme_primary, "#AABBCC");
        v.latitude = Some(90.000001);
        assert!(v.validate().is_err());
        v.latitude = Some(1.1234567);
        assert!(v.validate().is_err());
        assert!(SchoolInput::parse(json!({"name":"校","active":true})).is_err());
        for s in ["red", "var(--x)", "#ff00ff00", "#éé"] {
            assert!(color(s).is_err());
        }
    }
}
