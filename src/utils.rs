pub fn readable_bps(bps: f64) -> String {
    const DIVIDER: f64 = 1000.0;
    const MEASURES: [&str; 5] = ["B/s", "KB/s", "MB/s", "GB/s", "TB/s"];
    let mut bps = bps;

    for measure in MEASURES.iter() {
        if bps < DIVIDER {
            return format!("{:.2} {}", bps, measure);
        }

        bps /= DIVIDER;
    }

    String::from("fast boi (>1000 TB/s)")
}
