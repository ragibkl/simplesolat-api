use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

/// Retry an async function up to `max_retries` times with exponential backoff.
/// Returns the first Ok result, or the last error after all retries are exhausted.
pub async fn with_retries<F, Fut, T, E>(max_retries: u32, mut f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut last_err = None;
    for attempt in 0..=max_retries {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                if attempt < max_retries {
                    let delay = Duration::from_secs(1 << attempt); // 1s, 2s, 4s
                    tracing::warn!("retry attempt {}/{}, waiting {}s: {:?}", attempt + 1, max_retries, delay.as_secs(), e);
                    sleep(delay).await;
                }
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap())
}
