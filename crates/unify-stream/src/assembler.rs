use std::collections::BTreeMap;

use crate::packet::{Header, MAX_CHUNK_LEN};

const MAX_PENDING_FRAMES: usize = 6;

struct Pending {
    keyframe: bool,
    capture_us: u64,
    chunks: Vec<Option<Vec<u8>>>,
    received: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Assembled {
    pub frame_id: u32,
    pub keyframe: bool,
    pub capture_us: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct AssemblerOutput {
    pub frame: Option<Assembled>,
    pub request_keyframe: bool,
}

#[derive(Default)]
pub struct FrameAssembler {
    pending: BTreeMap<u32, Pending>,
    last_delivered: Option<u32>,
    awaiting_keyframe: bool,
    dropped_frames: u64,
}

impl FrameAssembler {
    pub fn new() -> Self {
        Self { awaiting_keyframe: true, ..Self::default() }
    }

    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames
    }

    fn abandon_before(&mut self, frame_id: u32) -> bool {
        let stale: Vec<u32> = self.pending.range(..frame_id).map(|(id, _)| *id).collect();
        for id in &stale {
            self.pending.remove(id);
            self.dropped_frames += 1;
        }
        !stale.is_empty()
    }

    pub fn push(&mut self, header: Header, data: &[u8]) -> AssemblerOutput {
        let mut output = AssemblerOutput::default();
        if data.len() > MAX_CHUNK_LEN || self.last_delivered.is_some_and(|last| header.frame_id <= last) {
            return output;
        }
        let entry = self.pending.entry(header.frame_id).or_insert_with(|| Pending {
            keyframe: header.keyframe,
            capture_us: header.capture_us,
            chunks: vec![None; usize::from(header.chunk_count)],
            received: 0,
        });
        if entry.chunks.len() != usize::from(header.chunk_count) {
            return output;
        }
        let slot = &mut entry.chunks[usize::from(header.chunk_index)];
        if slot.is_none() {
            *slot = Some(data.to_vec());
            entry.received += 1;
        }
        if entry.received == entry.chunks.len() {
            let keyframe = entry.keyframe;
            if self.awaiting_keyframe && !keyframe {
                self.pending.remove(&header.frame_id);
                output.request_keyframe = true;
                return output;
            }
            let lost_earlier = self.abandon_before(header.frame_id);
            let gap = self.last_delivered.is_some_and(|last| header.frame_id != last + 1);
            if (lost_earlier || gap) && !keyframe {
                self.awaiting_keyframe = true;
                self.pending.remove(&header.frame_id);
                self.dropped_frames += 1;
                output.request_keyframe = true;
                return output;
            }
            if let Some(done) = self.pending.remove(&header.frame_id) {
                self.awaiting_keyframe = false;
                self.last_delivered = Some(header.frame_id);
                output.frame = Some(Assembled {
                    frame_id: header.frame_id,
                    keyframe,
                    capture_us: done.capture_us,
                    data: done.chunks.into_iter().flatten().flatten().collect(),
                });
            }
            return output;
        }
        while self.pending.len() > MAX_PENDING_FRAMES {
            if let Some((&oldest, _)) = self.pending.iter().next() {
                self.pending.remove(&oldest);
                self.dropped_frames += 1;
                self.awaiting_keyframe = true;
                output.request_keyframe = true;
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(frame_id: u32, index: u16, count: u16, keyframe: bool) -> Header {
        Header { keyframe, sequence: 0, frame_id, chunk_index: index, chunk_count: count, capture_us: u64::from(frame_id) }
    }

    fn deliver(assembler: &mut FrameAssembler, frame_id: u32, keyframe: bool) -> AssemblerOutput {
        assembler.push(header(frame_id, 0, 1, keyframe), &[frame_id as u8])
    }

    #[test]
    fn joins_chunks_arriving_out_of_order() {
        let mut assembler = FrameAssembler::new();
        assert_eq!(assembler.push(header(1, 1, 2, true), b"world"), AssemblerOutput::default());
        let done = assembler.push(header(1, 0, 2, true), b"hello ").frame.unwrap();
        assert_eq!(done.data, b"hello world");
    }

    #[test]
    fn waits_for_a_keyframe_before_delivering() {
        let mut assembler = FrameAssembler::new();
        let first = deliver(&mut assembler, 1, false);
        assert!(first.frame.is_none() && first.request_keyframe);
        assert!(deliver(&mut assembler, 2, true).frame.is_some());
        assert!(deliver(&mut assembler, 3, false).frame.is_some());
    }

    #[test]
    fn lost_frame_requests_keyframe_and_blocks_until_it_arrives() {
        let mut assembler = FrameAssembler::new();
        deliver(&mut assembler, 1, true);
        assembler.push(header(2, 0, 2, false), b"partial");
        let after_loss = deliver(&mut assembler, 3, false);
        assert!(after_loss.frame.is_none() && after_loss.request_keyframe);
        assert!(deliver(&mut assembler, 4, false).frame.is_none());
        assert!(deliver(&mut assembler, 5, true).frame.is_some());
        assert_eq!(assembler.dropped_frames(), 2);
    }

    #[test]
    fn newer_keyframe_recovers_immediately_and_late_chunks_are_ignored() {
        let mut assembler = FrameAssembler::new();
        deliver(&mut assembler, 1, true);
        assert!(deliver(&mut assembler, 4, true).frame.is_some());
        assert_eq!(deliver(&mut assembler, 3, false), AssemblerOutput::default());
    }

    #[test]
    fn duplicate_chunks_do_not_complete_a_frame() {
        let mut assembler = FrameAssembler::new();
        assembler.push(header(1, 0, 2, true), b"a");
        assert!(assembler.push(header(1, 0, 2, true), b"a").frame.is_none());
    }

    #[test]
    fn inconsistent_chunk_count_is_ignored() {
        let mut assembler = FrameAssembler::new();
        assembler.push(header(1, 0, 2, true), b"a");
        assert!(assembler.push(header(1, 0, 1, true), b"b").frame.is_none());
    }

    #[test]
    fn bounds_pending_frames() {
        let mut assembler = FrameAssembler::new();
        let mut asked = false;
        for id in 1..=(MAX_PENDING_FRAMES as u32 + 2) {
            asked |= assembler.push(header(id, 0, 2, false), b"a").request_keyframe;
        }
        assert!(asked);
        assert!(assembler.pending.len() <= MAX_PENDING_FRAMES);
    }
}
