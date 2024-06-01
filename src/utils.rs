use std::time::{SystemTime, UNIX_EPOCH};

pub fn perf_trace(name: &str, cat: &str, ph: &str, ts: u128) {
    let pid = std::process::id();
    log::trace!("{{\"name\": \"{}\", \"cat\": \"{}\", \"ph\": \"{}\", \"pid\": {}, \"tid\": {}, \"ts\": {} }}, ", name, cat, ph, pid, 0, ts);
    // {"name": "Asub", "cat": "PERF", "ph": "B", "pid": 22630, "tid": 22630, "ts": 829},
}

pub fn get_micros() -> u128 {
    return SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_micros();
}
