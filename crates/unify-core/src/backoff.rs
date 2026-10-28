use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Backoff {
    attempt: u32,
    base_ms: u64,
    max_ms: u64,
}

impl Backoff {
    pub fn new(base_ms: u64, max_ms: u64) -> Self {
        Self { attempt: 0, base_ms: base_ms.max(1), max_ms: max_ms.max(base_ms.max(1)) }
    }

    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    pub fn next_delay(&mut self, jitter_unit: f64) -> Duration {
        let exponent = self.attempt.min(16);
        let raw = self.base_ms.saturating_mul(1u64 << exponent).min(self.max_ms);
        let jitter = if jitter_unit.is_finite() { jitter_unit.clamp(0.0, 1.0) } else { 0.5 };
        let scaled = (raw as f64 * (0.8 + 0.4 * jitter)).round() as u64;
        self.attempt = self.attempt.saturating_add(1);
        Duration::from_millis(scaled.clamp(1, self.max_ms + self.max_ms / 5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grows_exponentially_and_caps() {
        let mut backoff = Backoff::new(500, 10_000);
        let delays: Vec<u64> = (0..8).map(|_| backoff.next_delay(0.5).as_millis() as u64).collect();
        assert_eq!(delays, vec![500, 1000, 2000, 4000, 8000, 10_000, 10_000, 10_000]);
    }

    #[test]
    fn jitter_stays_within_twenty_percent() {
        let mut low = Backoff::new(1000, 10_000);
        let mut high = Backoff::new(1000, 10_000);
        assert_eq!(low.next_delay(0.0).as_millis(), 800);
        assert_eq!(high.next_delay(1.0).as_millis(), 1200);
        assert_eq!(Backoff::new(1000, 10_000).next_delay(f64::NAN).as_millis(), 1000);
    }

    #[test]
    fn reset_restarts_sequence() {
        let mut backoff = Backoff::new(100, 1000);
        backoff.next_delay(0.5);
        backoff.next_delay(0.5);
        backoff.reset();
        assert_eq!(backoff.attempt(), 0);
        assert_eq!(backoff.next_delay(0.5).as_millis(), 100);
    }
}
