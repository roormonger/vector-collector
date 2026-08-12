use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<RateLimiterInner>>,
}

struct RateLimiterInner {
    buckets: HashMap<String, Bucket>,
    rps: f64,
    burst: f64,
}

struct Bucket {
    tokens: f64,
    last: Instant,
}

impl RateLimiter {
    pub fn new(rps: f64) -> Self {
        let rps = rps.max(1.0);
        Self {
            inner: Arc::new(Mutex::new(RateLimiterInner {
                buckets: HashMap::new(),
                rps,
                burst: (rps * 2.0).max(5.0),
            })),
        }
    }

    pub fn set_rps(&self, rps: f64) {
        let rps = rps.max(1.0);
        let mut inner = self.inner.lock();
        inner.rps = rps;
        inner.burst = (rps * 2.0).max(5.0);
    }

    pub fn check(&self, key: &str) -> bool {
        let mut inner = self.inner.lock();
        let now = Instant::now();
        let rps = inner.rps;
        let burst = inner.burst;
        let bucket = inner.buckets.entry(key.to_string()).or_insert(Bucket {
            tokens: burst,
            last: now,
        });
        let elapsed = now.duration_since(bucket.last).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * rps).min(burst);
        bucket.last = now;
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}
