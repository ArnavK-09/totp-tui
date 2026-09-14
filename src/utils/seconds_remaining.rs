use std::time::{SystemTime, UNIX_EPOCH};

pub fn seconds_remaining(period: u32) -> u64 {
    let period = period.max(1) as u64;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    period - (now % period)
}
