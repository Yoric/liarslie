use std::future::Future;

use rand::Rng;

const MAX_RETRIES: u64 = 10;

async fn sleep(hint: u64) {
    let delay = std::time::Duration::new(
        hint * hint as u64,
        rand::thread_rng().gen_range(0, 1_000_000_000),
    );
    tokio::time::delay_for(delay)
        .await;
}

pub async fn retry_future_if<F, G, T, E, P>(mut f: F, should_try_again: G) -> Result<T, E>
where
    F: FnMut() -> P,
    P: Future<Output = Result<T, E>>,
    G: Fn(&E) -> bool,
{
    let mut error = None;
    for i in 0..MAX_RETRIES {
        match f().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                if should_try_again(&err) {
                    error = Some(err);
                    sleep(i).await;
                    continue;
                }
                return Err(err);
            }
        }
    }
    Err(error.unwrap())
}

pub async fn retry_closure_if<F, G, T, E>(mut f: F, should_try_again: G) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    G: Fn(&E) -> bool,
{
    let mut error = None;
    for i in 0..MAX_RETRIES {
        match f() {
            Ok(result) => return Ok(result),
            Err(err) => {
                if should_try_again(&err) {
                    error = Some(err);
                    sleep(i).await;
                    continue;
                }
                return Err(err);
            }
        }
    }
    Err(error.unwrap())
}

pub async fn retry_future<F, T, E, P>(f: F) -> Result<T, E>
where
    F: FnMut() -> P,
    P: Future<Output = Result<T, E>>,
{
    retry_future_if(f, |_| true).await
}

pub async fn retry_closure<F, T, E>(f: F) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    retry_closure_if(f, |_| true).await
}
