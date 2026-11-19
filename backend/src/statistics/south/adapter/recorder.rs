use crate::{
    business::{Actor, Result},
    statistics::{domain::ActivityEvent, south::port::ActivityRecorder},
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use uuid::Uuid;
#[derive(Clone)]
pub struct Recorder {
    inner: Arc<Inner>,
}
struct Inner {
    queue: Mutex<VecDeque<ActivityEvent>>,
    failures: AtomicU64,
    started: DateTime<Utc>,
    id: Uuid,
    capacity: usize,
}
impl Default for Recorder {
    fn default() -> Self {
        Self::new(4096)
    }
}
impl Recorder {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Inner {
                queue: Mutex::new(VecDeque::new()),
                failures: AtomicU64::new(0),
                started: Utc::now(),
                id: Uuid::new_v4(),
                capacity,
            }),
        }
    }
    pub fn event(
        &self,
        a: &Actor,
        school: Option<Uuid>,
        kind: &str,
        id: Option<Uuid>,
        post_id: Option<Uuid>,
    ) {
        if a.is_admin_browser() {
            return;
        }
        self.record(ActivityEvent {
            post_id,
            event_id: id.unwrap_or_else(Uuid::new_v4),
            account_id: a.id,
            school_id: school,
            kind: kind.into(),
            occurred_at: Utc::now(),
        });
    }
    pub async fn run(&self, pool: PgPool, config: crate::statistics::config::Config) {
        let mut registered = false;
        let mut run_id = self.inner.id;
        let mut run_start = self.inner.started;
        let mut failure_baseline = 0;
        let mut maintenance_at = std::time::Instant::now() - std::time::Duration::from_secs(3600);
        loop {
            if maintenance_at.elapsed().as_secs() >= 3600 {
                if maintain(&pool, &config).await.is_err() {
                    tracing::warn!("statistics maintenance failed");
                }
                maintenance_at = std::time::Instant::now();
            }
            if !registered {
                match sqlx::query("INSERT INTO statistics_activity_runs(id,started_at,sealed_at,heartbeat_at) VALUES($1,$2,$2,now())").bind(run_id).bind(run_start).execute(&pool).await{Ok(_)=>registered=true,Err(_)=>{self.inner.failures.fetch_add(1,Ordering::SeqCst);tracing::warn!("statistics collector registration failed");tokio::time::sleep(std::time::Duration::from_secs(2)).await;continue;}}
            }
            let (events, cutoff, failures) = {
                let mut queue = self.inner.queue.lock().unwrap_or_else(|e| e.into_inner());
                let count = queue.len().min(config.batch_size);
                let now = if count < queue.len() {
                    queue[count].occurred_at
                } else {
                    Utc::now()
                };
                (
                    queue.drain(..count).collect::<Vec<_>>(),
                    now,
                    self.inner.failures.load(Ordering::SeqCst),
                )
            };
            let mut persisted = false;
            for _ in 0..3 {
                if self
                    .flush(&pool, &events, cutoff, run_id, failures > failure_baseline)
                    .await
                    .is_ok()
                {
                    persisted = true;
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            if !persisted {
                self.inner.failures.fetch_add(1, Ordering::SeqCst);
                tracing::warn!("statistics activity persistence failed; coverage is incomplete");
            }
            // Persist and close a failed interval before beginning a clean one. Failures
            // occurring after the queue cutoff remain visible through the sequence counter.
            if persisted
                && failures > failure_baseline
                && sqlx::query("UPDATE statistics_activity_runs SET closed_at=$2 WHERE id=$1")
                    .bind(run_id)
                    .bind(cutoff)
                    .execute(&pool)
                    .await
                    .is_ok()
            {
                run_id = Uuid::new_v4();
                run_start = cutoff;
                failure_baseline = failures;
                registered = false;
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
    async fn flush(
        &self,
        pool: &PgPool,
        events: &[ActivityEvent],
        cutoff: DateTime<Utc>,
        run_id: Uuid,
        failed: bool,
    ) -> Result<()> {
        let mut tx = pool.begin().await?;
        sqlx::query("SET LOCAL statement_timeout='3s'")
            .execute(&mut *tx)
            .await?;
        for e in events {
            let valid:bool=sqlx::query_scalar("WITH inserted AS (INSERT INTO statistics_activity_events(event_id,account_id,school_id,kind,occurred_at,post_id) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING RETURNING event_id) SELECT EXISTS(SELECT 1 FROM inserted) OR EXISTS(SELECT 1 FROM statistics_activity_events WHERE event_id=$1 AND account_id=$2 AND school_id IS NOT DISTINCT FROM $3 AND kind=$4 AND post_id IS NOT DISTINCT FROM $6)").bind(e.event_id).bind(e.account_id).bind(e.school_id).bind(&e.kind).bind(e.occurred_at).bind(e.post_id).fetch_one(&mut *tx).await?;
            if !valid {
                tracing::warn!("conflicting statistics activity identity rejected");
            }
        }
        sqlx::query("UPDATE statistics_activity_runs SET sealed_at=$2,heartbeat_at=now(),failed=failed OR $3 WHERE id=$1").bind(run_id).bind(cutoff).bind(failed).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
}
impl ActivityRecorder for Recorder {
    fn record(&self, mut event: ActivityEvent) {
        let mut queue = self.inner.queue.lock().unwrap_or_else(|e| e.into_inner());
        if queue.len() >= self.inner.capacity {
            self.inner.failures.fetch_add(1, Ordering::SeqCst);
            tracing::warn!("statistics activity queue full; coverage is incomplete");
            return;
        }
        event.occurred_at = Utc::now();
        queue.push_back(event);
    }
}
async fn maintain(pool: &PgPool, c: &crate::statistics::config::Config) -> Result<()> {
    if let Some(start) = c.activity_complete_from {
        let family = "activity";
        sqlx::query("INSERT INTO statistics_coverage(id,family,start_at,status,reason) SELECT $1,$2,$3,'complete','operator_verified' WHERE NOT EXISTS(SELECT 1 FROM statistics_coverage WHERE family=$2 AND start_at=$3 AND reason='operator_verified')").bind(Uuid::new_v4()).bind(family).bind(start).execute(pool).await?;
    }
    sqlx::query("DELETE FROM statistics_snapshots WHERE expires_at<now()")
        .execute(pool)
        .await?;
    let cutoff:chrono::DateTime<Utc>=sqlx::query_scalar("SELECT date_trunc('month',now() AT TIME ZONE 'Asia/Shanghai') AT TIME ZONE 'Asia/Shanghai' - make_interval(months=>$1)").bind(c.retention_months).fetch_one(pool).await?;
    sqlx::query("DELETE FROM statistics_activity_events WHERE event_id IN (SELECT event_id FROM statistics_activity_events WHERE occurred_at<$1 LIMIT 10000)").bind(cutoff).execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overflow_is_bounded_and_marks_gap() {
        let r = Recorder::new(1);
        for _ in 0..2 {
            r.record(ActivityEvent {
                post_id: None,
                event_id: Uuid::new_v4(),
                account_id: Uuid::new_v4(),
                school_id: None,
                kind: "login".into(),
                occurred_at: Utc::now(),
            });
        }
        assert_eq!(r.inner.queue.lock().unwrap().len(), 1);
        assert_eq!(r.inner.failures.load(Ordering::SeqCst), 1);
    }
}
