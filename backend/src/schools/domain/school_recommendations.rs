use crate::business::{Result, require};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const LIMIT: i64 = 5;

#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Scene {
    Binding,
    Browsing,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub latitude: f64,
    pub longitude: f64,
    pub coordinate_system: String,
    pub scene: Scene,
}

impl Query {
    pub fn validate(&self) -> Result<()> {
        require(self.coordinate_system == "gcj02", "仅支持 GCJ-02 坐标")?;
        for (value, maximum) in [(self.latitude, 90.0), (self.longitude, 180.0)] {
            require(
                value.is_finite()
                    && value.abs() <= maximum
                    && (value * 1e6 - (value * 1e6).round()).abs() < 0.00001,
                "坐标范围无效或超过六位小数",
            )?;
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnboardingQuery {
    pub ticket: String,
    pub latitude: f64,
    pub longitude: f64,
    pub coordinate_system: String,
}

impl OnboardingQuery {
    pub fn query(&self) -> Result<Query> {
        require(
            self.ticket.len() == 64 && self.ticket.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "入驻凭据不合法",
        )?;
        let query = Query {
            latitude: self.latitude,
            longitude: self.longitude,
            coordinate_system: self.coordinate_system.clone(),
            scene: Scene::Binding,
        };
        query.validate()?;
        Ok(query)
    }
}

#[derive(Serialize)]
pub struct Recommendation {
    #[serde(flatten)]
    pub school: Value,
    pub distance_m: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_coordinates_and_rejects_spoofed_fields() {
        let valid =
            json!({"latitude":0,"longitude":180,"coordinate_system":"gcj02","scene":"binding"});
        assert!(
            serde_json::from_value::<Query>(valid.clone())
                .unwrap()
                .validate()
                .is_ok()
        );
        for (field, value) in [
            ("latitude", json!(90.1)),
            ("longitude", json!(-180.1)),
            ("latitude", json!(1.0000001)),
            ("coordinate_system", json!("wgs84")),
        ] {
            let mut input = valid.clone();
            input[field] = value;
            assert!(
                serde_json::from_value::<Query>(input)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
        for field in ["limit", "radius", "account_id", "authorized"] {
            let mut input = valid.clone();
            input[field] = json!(5);
            assert!(serde_json::from_value::<Query>(input).is_err());
        }
        let mut input: Query = serde_json::from_value(valid).unwrap();
        input.latitude = f64::NAN;
        assert!(input.validate().is_err());
    }
}
