pub fn frame_count<const FRAME_RATE: u64>(frame_count: u64) -> time::Duration {
    let secs = frame_count / FRAME_RATE;
    let nanos = (frame_count % FRAME_RATE) * 1_000_000_000 / FRAME_RATE;
    time::Duration::new(secs as _, nanos as _)
}
