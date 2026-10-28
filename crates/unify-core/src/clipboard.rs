use sha2::{Digest, Sha256};
use thiserror::Error;
use unify_proto::{CLIPBOARD_CHUNK_LEN, MAX_CLIPBOARD_TEXT_LEN};

pub type ContentHash = [u8; 32];

pub fn text_hash(text: &str) -> ContentHash {
    Sha256::digest(text.as_bytes()).into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    Disabled,
    Concealed,
    Empty,
    TooLarge,
    Echo,
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalDecision {
    Send { hash: ContentHash },
    Skip(SkipReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteDecision {
    Apply,
    AlreadyCurrent,
}

#[derive(Debug, Default)]
pub struct ClipboardSync {
    last_sent: Option<ContentHash>,
    last_applied: Option<ContentHash>,
}

impl ClipboardSync {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_local_change(&mut self, text: &str, concealed: bool, force: bool) -> LocalDecision {
        if concealed {
            return LocalDecision::Skip(SkipReason::Concealed);
        }
        if text.is_empty() {
            return LocalDecision::Skip(SkipReason::Empty);
        }
        if text.len() > MAX_CLIPBOARD_TEXT_LEN {
            return LocalDecision::Skip(SkipReason::TooLarge);
        }
        let hash = text_hash(text);
        if !force && self.last_applied == Some(hash) {
            return LocalDecision::Skip(SkipReason::Echo);
        }
        if !force && self.last_sent == Some(hash) {
            return LocalDecision::Skip(SkipReason::Duplicate);
        }
        self.last_sent = Some(hash);
        self.last_applied = None;
        LocalDecision::Send { hash }
    }

    pub fn on_remote_text(&mut self, hash: ContentHash) -> RemoteDecision {
        if self.last_sent == Some(hash) || self.last_applied == Some(hash) {
            return RemoteDecision::AlreadyCurrent;
        }
        self.last_applied = Some(hash);
        self.last_sent = None;
        RemoteDecision::Apply
    }

    pub fn forget_sent(&mut self) {
        self.last_sent = None;
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AssemblyError {
    #[error("clipboard transfer too large")]
    TooLarge,
    #[error("empty clipboard transfer")]
    Empty,
    #[error("chunk for unknown transfer")]
    UnknownTransfer,
    #[error("chunk out of order")]
    OutOfOrder,
    #[error("chunk exceeds declared length")]
    Overflow,
    #[error("content hash mismatch")]
    HashMismatch,
    #[error("content is not valid UTF-8")]
    InvalidUtf8,
}

#[derive(Debug)]
struct Transfer {
    id: u64,
    total_len: usize,
    hash: ContentHash,
    buffer: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct ClipboardAssembler {
    current: Option<Transfer>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AssembledText {
    pub text: String,
    pub hash: ContentHash,
}

impl ClipboardAssembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn in_progress(&self) -> bool {
        self.current.is_some()
    }

    pub fn reset(&mut self) {
        self.current = None;
    }

    pub fn begin(&mut self, id: u64, total_len: u32, hash: ContentHash) -> Result<(), AssemblyError> {
        self.current = None;
        let total_len = total_len as usize;
        if total_len == 0 {
            return Err(AssemblyError::Empty);
        }
        if total_len > MAX_CLIPBOARD_TEXT_LEN {
            return Err(AssemblyError::TooLarge);
        }
        self.current = Some(Transfer { id, total_len, hash, buffer: Vec::with_capacity(total_len.min(1 << 20)) });
        Ok(())
    }

    pub fn chunk(&mut self, id: u64, offset: u32, data: &[u8]) -> Result<Option<AssembledText>, AssemblyError> {
        let transfer = match self.current.as_mut() {
            Some(transfer) if transfer.id == id => transfer,
            _ => return Err(AssemblyError::UnknownTransfer),
        };
        if offset as usize != transfer.buffer.len() {
            self.current = None;
            return Err(AssemblyError::OutOfOrder);
        }
        if transfer.buffer.len() + data.len() > transfer.total_len {
            self.current = None;
            return Err(AssemblyError::Overflow);
        }
        transfer.buffer.extend_from_slice(data);
        if transfer.buffer.len() < transfer.total_len {
            return Ok(None);
        }
        let Some(done) = self.current.take() else {
            return Err(AssemblyError::UnknownTransfer);
        };
        let actual: ContentHash = Sha256::digest(&done.buffer).into();
        if actual != done.hash {
            return Err(AssemblyError::HashMismatch);
        }
        let text = String::from_utf8(done.buffer).map_err(|_| AssemblyError::InvalidUtf8)?;
        Ok(Some(AssembledText { text, hash: done.hash }))
    }
}

pub fn chunk_offsets(len: usize) -> impl Iterator<Item = (usize, usize)> {
    (0..len).step_by(CLIPBOARD_CHUNK_LEN).map(move |start| (start, (start + CLIPBOARD_CHUNK_LEN).min(len)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transfer(assembler: &mut ClipboardAssembler, id: u64, text: &str) -> Result<Option<AssembledText>, AssemblyError> {
        assembler.begin(id, text.len() as u32, text_hash(text))?;
        let bytes = text.as_bytes();
        let mut last = None;
        for (start, end) in chunk_offsets(bytes.len()) {
            last = assembler.chunk(id, start as u32, &bytes[start..end])?;
        }
        Ok(last)
    }

    #[test]
    fn sends_new_local_text_once() {
        let mut sync = ClipboardSync::new();
        assert!(matches!(sync.on_local_change("merhaba", false, false), LocalDecision::Send { .. }));
        assert_eq!(sync.on_local_change("merhaba", false, false), LocalDecision::Skip(SkipReason::Duplicate));
    }

    #[test]
    fn applied_remote_text_is_not_echoed_back() {
        let mut sync = ClipboardSync::new();
        let text = "Windows'tan geldi";
        assert_eq!(sync.on_remote_text(text_hash(text)), RemoteDecision::Apply);
        assert_eq!(sync.on_local_change(text, false, false), LocalDecision::Skip(SkipReason::Echo));
    }

    #[test]
    fn two_peers_converge_without_loop() {
        let mut mac = ClipboardSync::new();
        let mut win = ClipboardSync::new();
        let text = "döngü testi";
        let LocalDecision::Send { hash } = mac.on_local_change(text, false, false) else { panic!() };
        assert_eq!(win.on_remote_text(hash), RemoteDecision::Apply);
        assert_eq!(win.on_local_change(text, false, false), LocalDecision::Skip(SkipReason::Echo));
        assert_eq!(mac.on_remote_text(hash), RemoteDecision::AlreadyCurrent);
    }

    #[test]
    fn same_text_can_be_resent_after_other_content_in_between() {
        let mut sync = ClipboardSync::new();
        assert!(matches!(sync.on_local_change("a", false, false), LocalDecision::Send { .. }));
        assert!(matches!(sync.on_local_change("b", false, false), LocalDecision::Send { .. }));
        assert!(matches!(sync.on_local_change("a", false, false), LocalDecision::Send { .. }));
    }

    #[test]
    fn alternating_copies_on_both_sides_always_propagate() {
        let mut mac = ClipboardSync::new();
        let mut win = ClipboardSync::new();
        let LocalDecision::Send { hash: a } = win.on_local_change("a", false, false) else { panic!() };
        assert_eq!(mac.on_remote_text(a), RemoteDecision::Apply);
        let LocalDecision::Send { hash: b } = mac.on_local_change("b", false, false) else { panic!() };
        assert_eq!(win.on_remote_text(b), RemoteDecision::Apply);
        let LocalDecision::Send { hash: a2 } = win.on_local_change("a", false, false) else { panic!("a must be resent") };
        assert_eq!(mac.on_remote_text(a2), RemoteDecision::Apply);
    }

    #[test]
    fn force_bypasses_dedup() {
        let mut sync = ClipboardSync::new();
        sync.on_local_change("x", false, false);
        assert!(matches!(sync.on_local_change("x", false, true), LocalDecision::Send { .. }));
    }

    #[test]
    fn skips_concealed_empty_and_oversized() {
        let mut sync = ClipboardSync::new();
        assert_eq!(sync.on_local_change("parola", true, false), LocalDecision::Skip(SkipReason::Concealed));
        assert_eq!(sync.on_local_change("", false, false), LocalDecision::Skip(SkipReason::Empty));
        let big = "a".repeat(MAX_CLIPBOARD_TEXT_LEN + 1);
        assert_eq!(sync.on_local_change(&big, false, false), LocalDecision::Skip(SkipReason::TooLarge));
    }

    #[test]
    fn assembles_multi_chunk_unicode_text_with_line_endings() {
        let text = "İstanbul ğüşıöç ÇĞÜŞİÖ\r\nsatır 2\nsatır 3 🙂\u{0301}".repeat(2000);
        let mut assembler = ClipboardAssembler::new();
        let result = transfer(&mut assembler, 1, &text).unwrap().unwrap();
        assert_eq!(result.text, text);
        assert!(!assembler.in_progress());
    }

    #[test]
    fn rejects_hash_mismatch() {
        let mut assembler = ClipboardAssembler::new();
        assembler.begin(1, 3, text_hash("abd")).unwrap();
        assert_eq!(assembler.chunk(1, 0, b"abc"), Err(AssemblyError::HashMismatch));
    }

    #[test]
    fn rejects_out_of_order_and_overflow_and_unknown() {
        let mut assembler = ClipboardAssembler::new();
        assembler.begin(1, 4, text_hash("abcd")).unwrap();
        assert_eq!(assembler.chunk(2, 0, b"ab"), Err(AssemblyError::UnknownTransfer));
        assert_eq!(assembler.chunk(1, 2, b"ab"), Err(AssemblyError::OutOfOrder));
        assembler.begin(1, 4, text_hash("abcd")).unwrap();
        assert_eq!(assembler.chunk(1, 0, b"abcde"), Err(AssemblyError::Overflow));
        assert_eq!(assembler.begin(1, 0, [0; 32]), Err(AssemblyError::Empty));
        assert_eq!(assembler.begin(1, (MAX_CLIPBOARD_TEXT_LEN + 1) as u32, [0; 32]), Err(AssemblyError::TooLarge));
    }

    #[test]
    fn rejects_invalid_utf8_even_with_matching_hash() {
        let bytes = [0xC3u8, 0x28];
        let hash: ContentHash = Sha256::digest(bytes).into();
        let mut assembler = ClipboardAssembler::new();
        assembler.begin(5, 2, hash).unwrap();
        assert_eq!(assembler.chunk(5, 0, &bytes), Err(AssemblyError::InvalidUtf8));
    }

    #[test]
    fn newer_transfer_replaces_in_progress_one() {
        let mut assembler = ClipboardAssembler::new();
        assembler.begin(1, 10, [0; 32]).unwrap();
        assert_eq!(transfer(&mut assembler, 2, "yeni").unwrap().unwrap().text, "yeni");
    }

    #[test]
    fn chunk_offsets_cover_input_exactly() {
        assert_eq!(chunk_offsets(0).count(), 0);
        let len = CLIPBOARD_CHUNK_LEN * 2 + 5;
        let chunks: Vec<_> = chunk_offsets(len).collect();
        assert_eq!(chunks, vec![(0, CLIPBOARD_CHUNK_LEN), (CLIPBOARD_CHUNK_LEN, CLIPBOARD_CHUNK_LEN * 2), (CLIPBOARD_CHUNK_LEN * 2, len)]);
    }
}
