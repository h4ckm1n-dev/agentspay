//! Sliding-window per-IP rate limiter backed by Redis (or an in-memory
//! fallback when `AGENTSPAY_REDIS_URL` is unset).

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::Mutex;

pub struct RateLimit {
    inner: Backend,
}

enum Backend {
    Memory(Mutex<HashMap<String, Vec<Instant>>>),
    Redis(redis::aio::ConnectionManager),
}

impl RateLimit {
    pub fn in_memory() -> Self {
        Self {
            inner: Backend::Memory(Mutex::new(HashMap::new())),
        }
    }

    pub async fn redis(url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(url)?;
        let conn = redis::aio::ConnectionManager::new(client).await?;
        Ok(Self {
            inner: Backend::Redis(conn),
        })
    }

    /// Returns `Ok(())` if the call is allowed, `Err(retry_after_secs)` if
    /// the bucket is full.
    pub async fn check(&self, bucket: &str, max: u32, window: Duration) -> Result<(), u64> {
        match &self.inner {
            Backend::Memory(m) => {
                let now = Instant::now();
                let mut map = m.lock().await;
                let bucket_vec = map.entry(bucket.to_string()).or_default();
                bucket_vec.retain(|t| now.duration_since(*t) < window);
                if bucket_vec.len() as u32 >= max {
                    let oldest = bucket_vec[0];
                    let elapsed = now.duration_since(oldest);
                    let retry = window.saturating_sub(elapsed).as_secs().max(1);
                    return Err(retry);
                }
                bucket_vec.push(now);
                Ok(())
            }
            Backend::Redis(conn) => {
                let mut conn = conn.clone();
                let key = format!("ratelimit:{bucket}");
                let count: u32 = redis::cmd("INCR")
                    .arg(&key)
                    .query_async(&mut conn)
                    .await
                    .map_err(|_| window.as_secs())?;
                if count == 1 {
                    let _: redis::RedisResult<()> = redis::cmd("EXPIRE")
                        .arg(&key)
                        .arg(window.as_secs())
                        .query_async(&mut conn)
                        .await;
                }
                if count > max {
                    let ttl: i64 = redis::cmd("TTL")
                        .arg(&key)
                        .query_async(&mut conn)
                        .await
                        .unwrap_or(window.as_secs() as i64);
                    return Err(ttl.max(1) as u64);
                }
                Ok(())
            }
        }
    }
}

pub type SharedRateLimit = Arc<RateLimit>;

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn allows_under_cap_then_rejects_over() {
        let rl = RateLimit::in_memory();
        for _ in 0..3 {
            rl.check("ip:1.2.3.4", 3, Duration::from_secs(60))
                .await
                .unwrap();
        }
        let err = rl
            .check("ip:1.2.3.4", 3, Duration::from_secs(60))
            .await
            .unwrap_err();
        assert!(err > 0 && err <= 60);
    }
}
