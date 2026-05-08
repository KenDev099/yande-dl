use crate::error::CoreError;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl RetryPolicy {
    /// 3 attempts, exponential backoff 2s -> 4s, capped at 30s. For network
    /// requests and transient 5xx.
    pub fn standard() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 2_000,
            max_delay_ms: 30_000,
        }
    }

    /// 2 attempts, fixed 1s delay. For per-image MD5 mismatch — likely a
    /// transmission flip rather than upstream corruption, but only worth
    /// one retry.
    pub fn fast() -> Self {
        Self {
            max_attempts: 2,
            base_delay_ms: 1_000,
            max_delay_ms: 1_000,
        }
    }
}

/// Run `op` with exponential backoff. Retries transient network errors,
/// 5xx, and 429 (honoring `Retry-After`). Does NOT retry `Cancelled`,
/// `Md5Mismatch`, or `Parse`.
pub async fn with_backoff<F, Fut, T>(policy: &RetryPolicy, mut op: F) -> Result<T, CoreError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, CoreError>>,
{
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) if attempt >= policy.max_attempts => return Err(e),
            Err(e) => {
                let delay_ms = match &e {
                    CoreError::Cancelled | CoreError::Md5Mismatch { .. } | CoreError::Parse(_) => {
                        return Err(e);
                    }
                    CoreError::RateLimited { retry_after_secs } => retry_after_secs
                        .map(|s| s.saturating_mul(1000))
                        .unwrap_or(policy.base_delay_ms)
                        .min(policy.max_delay_ms),
                    _ => {
                        // Cap the shift to avoid overflow on pathological inputs.
                        let shift = (attempt - 1).min(20);
                        let exp = policy.base_delay_ms.saturating_mul(1u64 << shift);
                        exp.min(policy.max_delay_ms)
                    }
                };
                tracing::warn!(attempt, delay_ms, error = %e, "retrying");
                sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use tokio::time::Instant;

    fn fast_policy_no_wait() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 5,
            base_delay_ms: 1,
            max_delay_ms: 5,
        }
    }

    #[tokio::test]
    async fn succeeds_on_first_attempt() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_c = calls.clone();
        let res: Result<i32, CoreError> = with_backoff(&fast_policy_no_wait(), move || {
            let calls = calls_c.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(42)
            }
        })
        .await;
        assert_eq!(res.unwrap(), 42);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retries_until_success() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_c = calls.clone();
        let res: Result<i32, CoreError> = with_backoff(&fast_policy_no_wait(), move || {
            let calls = calls_c.clone();
            async move {
                let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
                if n < 3 {
                    Err(CoreError::Server { status: 503 })
                } else {
                    Ok(7)
                }
            }
        })
        .await;
        assert_eq!(res.unwrap(), 7);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn gives_up_after_max_attempts() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_c = calls.clone();
        let res: Result<i32, CoreError> = with_backoff(&fast_policy_no_wait(), move || {
            let calls = calls_c.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Err(CoreError::Server { status: 500 })
            }
        })
        .await;
        assert!(matches!(res, Err(CoreError::Server { status: 500 })));
        assert_eq!(calls.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn does_not_retry_cancelled() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_c = calls.clone();
        let res: Result<i32, CoreError> = with_backoff(&fast_policy_no_wait(), move || {
            let calls = calls_c.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Err(CoreError::Cancelled)
            }
        })
        .await;
        assert!(matches!(res, Err(CoreError::Cancelled)));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn does_not_retry_md5_mismatch() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_c = calls.clone();
        let res: Result<i32, CoreError> = with_backoff(&fast_policy_no_wait(), move || {
            let calls = calls_c.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Err(CoreError::Md5Mismatch {
                    expected: "a".into(),
                    actual: "b".into(),
                })
            }
        })
        .await;
        assert!(matches!(res, Err(CoreError::Md5Mismatch { .. })));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn does_not_retry_parse() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_c = calls.clone();
        let res: Result<i32, CoreError> = with_backoff(&fast_policy_no_wait(), move || {
            let calls = calls_c.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Err(CoreError::Parse("bad json".into()))
            }
        })
        .await;
        assert!(matches!(res, Err(CoreError::Parse(_))));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn rate_limit_respects_retry_after_capped() {
        // policy max_delay_ms = 5 → even if Retry-After says 60s, we cap at 5ms.
        let calls = Arc::new(AtomicU32::new(0));
        let calls_c = calls.clone();
        let start = Instant::now();
        let _ = with_backoff(&fast_policy_no_wait(), move || {
            let calls = calls_c.clone();
            async move {
                let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
                if n < 2 {
                    Err::<i32, _>(CoreError::RateLimited {
                        retry_after_secs: Some(60),
                    })
                } else {
                    Ok(0)
                }
            }
        })
        .await;
        // Should not have actually waited 60 seconds.
        assert!(start.elapsed() < Duration::from_secs(1));
    }
}
