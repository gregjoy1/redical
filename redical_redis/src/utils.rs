use std::{
    sync::mpsc,
    thread,
    time::Duration,
};

#[derive(Debug)]
pub struct TimeoutError;

pub fn run_with_timeout<F, T>(function: F, timeout: Duration) -> Result<T, TimeoutError>
where
    F: FnOnce() -> T + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    let (tx, rx) = mpsc::channel();

    let _ = thread::spawn(move || {
        let result = function();

        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => Ok(result),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(TimeoutError),
        Err(mpsc::RecvTimeoutError::Disconnected) => unreachable!(),
    }
}
