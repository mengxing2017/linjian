use chrono::{DateTime, Utc};
#[derive(Clone, Debug)]
pub struct Config {
    pub concurrency: usize,
    pub timeout_ms: u64,
    pub queue_capacity: usize,
    pub batch_size: usize,
    pub snapshot_seconds: i64,
    pub retention_months: i32,
    pub content_complete_from: Option<DateTime<Utc>>,
    pub registration_complete_from: Option<DateTime<Utc>>,
    pub activity_complete_from: Option<DateTime<Utc>>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            concurrency: 2,
            timeout_ms: 15000,
            queue_capacity: 4096,
            batch_size: 256,
            snapshot_seconds: 600,
            retention_months: 25,
            content_complete_from: None,
            registration_complete_from: None,
            activity_complete_from: None,
        }
    }
}
impl Config {
    pub fn from_env() -> Result<Self, String> {
        fn number(name: &str, default: u64, min: u64, max: u64) -> Result<u64, String> {
            let key = format!("LINJIAN_STATISTICS_{name}");
            let n = match std::env::var(&key) {
                Ok(s) => s
                    .parse::<u64>()
                    .map_err(|_| format!("{key} must be an integer"))?,
                Err(_) => default,
            };
            if !(min..=max).contains(&n) {
                return Err(format!("{key} is outside the supported range"));
            }
            Ok(n)
        }
        fn timestamp(name: &str) -> Result<Option<DateTime<Utc>>, String> {
            let key = format!("LINJIAN_STATISTICS_{name}_COMPLETE_FROM");
            std::env::var(&key)
                .ok()
                .filter(|s| !s.trim().is_empty())
                .map(|s| {
                    s.parse::<DateTime<Utc>>()
                        .map_err(|_| format!("{key} must be an RFC3339 timestamp"))
                })
                .transpose()
        }
        let c = Self {
            concurrency: number("CONCURRENCY", 2, 1, 8)? as usize,
            timeout_ms: number("TIMEOUT_MS", 15000, 1000, 60000)?,
            queue_capacity: number("QUEUE_CAPACITY", 4096, 1, 100000)? as usize,
            batch_size: number("BATCH_SIZE", 256, 1, 10000)? as usize,
            snapshot_seconds: number("SNAPSHOT_SECONDS", 600, 60, 3600)? as i64,
            retention_months: number("RETENTION_MONTHS", 25, 25, 120)? as i32,
            content_complete_from: timestamp("CONTENT")?,
            registration_complete_from: timestamp("REGISTRATION")?,
            activity_complete_from: timestamp("ACTIVITY")?,
        };
        for t in [
            c.content_complete_from,
            c.registration_complete_from,
            c.activity_complete_from,
        ]
        .into_iter()
        .flatten()
        {
            if t > Utc::now() {
                return Err("statistics coverage cannot start in the future".into());
            }
        }
        Ok(c)
    }
}
