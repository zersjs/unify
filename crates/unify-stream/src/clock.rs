use std::collections::VecDeque;

const OFFSET_SAMPLES: usize = 32;

#[derive(Debug, Default)]
pub struct ClockSync {
    samples: VecDeque<(u64, i64)>,
}

impl ClockSync {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_sample(&mut self, sent_us: u64, peer_us: u64, received_us: u64) {
        if received_us < sent_us {
            return;
        }
        let rtt = received_us - sent_us;
        let midpoint = sent_us + rtt / 2;
        self.samples.push_back((rtt, peer_us as i64 - midpoint as i64));
        if self.samples.len() > OFFSET_SAMPLES {
            self.samples.pop_front();
        }
    }

    pub fn peer_minus_local_us(&self) -> Option<i64> {
        self.samples.iter().min_by_key(|(rtt, _)| *rtt).map(|(_, offset)| *offset)
    }

    pub fn to_local_us(&self, peer_us: u64) -> Option<u64> {
        self.peer_minus_local_us().map(|offset| (peer_us as i64 - offset).max(0) as u64)
    }
}

#[derive(Debug)]
pub struct LatencyWindow {
    samples: VecDeque<u32>,
    capacity: usize,
}

impl LatencyWindow {
    pub fn new(capacity: usize) -> Self {
        Self { samples: VecDeque::with_capacity(capacity), capacity: capacity.max(1) }
    }

    pub fn add(&mut self, micros: u32) {
        if self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(micros);
    }

    pub fn percentile(&self, percent: u32) -> Option<u32> {
        if self.samples.is_empty() {
            return None;
        }
        let mut sorted: Vec<u32> = self.samples.iter().copied().collect();
        sorted.sort_unstable();
        let rank = (sorted.len() - 1) * percent.min(100) as usize / 100;
        Some(sorted[rank])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_offset_from_lowest_rtt_sample() {
        let mut clock = ClockSync::new();
        clock.add_sample(1_000, 501_500, 2_000);
        clock.add_sample(2_000, 503_000, 12_000);
        assert_eq!(clock.peer_minus_local_us(), Some(500_000));
        assert_eq!(clock.to_local_us(600_000), Some(100_000));
    }

    #[test]
    fn ignores_impossible_samples_and_reports_none_when_empty() {
        let mut clock = ClockSync::new();
        clock.add_sample(10, 0, 5);
        assert_eq!(clock.peer_minus_local_us(), None);
    }

    #[test]
    fn computes_percentiles_over_a_bounded_window() {
        let mut window = LatencyWindow::new(100);
        assert_eq!(window.percentile(50), None);
        for value in 1..=200 {
            window.add(value);
        }
        assert_eq!(window.percentile(0), Some(101));
        assert_eq!(window.percentile(50), Some(150));
        assert_eq!(window.percentile(100), Some(200));
    }
}
