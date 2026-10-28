use std::sync::Arc;

use unify_core::clipboard::{chunk_offsets, text_hash, AssemblyError, LocalDecision, RemoteDecision, SkipReason};
use unify_platform::ClipboardText;
use unify_proto::{ClipboardFormat, Message};

use crate::actor::{now_ms, Actor, Event};
use crate::status::{ClipboardActivity, TransferDirection};

fn skip_code(reason: SkipReason) -> &'static str {
    match reason {
        SkipReason::Disabled => "disabled",
        SkipReason::Concealed => "concealed",
        SkipReason::Empty => "empty",
        SkipReason::TooLarge => "too_large",
        SkipReason::Echo => "echo",
        SkipReason::Duplicate => "duplicate",
    }
}

impl Actor {
    pub fn sync_clipboard_watch(&mut self) {
        let wanted = self.config.settings.clipboard_sync;
        if !wanted {
            if let Some(watch) = self.clipboard_watch.take() {
                watch.stop();
            }
            self.pending_clipboard = None;
            self.assembler.reset();
            self.clipboard_activity = ClipboardActivity::Idle;
            self.dirty = true;
            return;
        }
        if self.clipboard_watch.is_some() {
            return;
        }
        let events = self.events_tx.clone();
        let sink = Arc::new(move || {
            let _ = events.send(Event::ClipboardChanged);
        });
        match self.platform.start_clipboard_watch(sink) {
            Ok(handle) => self.clipboard_watch = Some(handle),
            Err(err) => {
                tracing::error!(error = %err, "clipboard monitoring unavailable");
                self.set_error("clipboard_watch_failed");
            }
        }
        self.dirty = true;
    }

    pub fn request_clipboard_read(&mut self, force: bool) {
        if !self.config.settings.clipboard_sync {
            if force {
                self.last_skip = Some(skip_code(SkipReason::Disabled));
                self.dirty = true;
            }
            return;
        }
        if self.clipboard_read_in_flight {
            self.clipboard_reread = Some(self.clipboard_reread.unwrap_or(false) || force);
            return;
        }
        self.clipboard_read_in_flight = true;
        let platform = self.platform.clone();
        let events = self.events_tx.clone();
        tokio::task::spawn_blocking(move || {
            let result = platform.read_clipboard().map_err(|err| err.to_string());
            let _ = events.send(Event::ClipboardRead { result, force });
        });
    }

    pub fn on_clipboard_read(&mut self, result: Result<Option<ClipboardText>, String>, force: bool) {
        self.clipboard_read_in_flight = false;
        if let Some(again) = self.clipboard_reread.take() {
            self.request_clipboard_read(again);
        }
        let content = match result {
            Ok(Some(content)) => content,
            Ok(None) => return,
            Err(err) => {
                tracing::warn!(error = %err, "clipboard read failed");
                self.set_error("clipboard_read_failed");
                return;
            }
        };
        match self.clipboard_sync.on_local_change(&content.text, content.concealed, force) {
            LocalDecision::Send { .. } => {
                if self.active.is_some() {
                    self.send_clipboard(content.text);
                } else {
                    self.pending_clipboard = Some(content.text);
                }
            }
            LocalDecision::Skip(reason) => {
                if !matches!(reason, SkipReason::Echo | SkipReason::Duplicate) {
                    self.last_skip = Some(skip_code(reason));
                    self.dirty = true;
                }
            }
        }
    }

    pub fn flush_pending_clipboard(&mut self) {
        if let Some(text) = self.pending_clipboard.take() {
            self.send_clipboard(text);
        }
    }

    fn send_clipboard(&mut self, text: String) {
        let Some(outbound) = self.active.and_then(|id| self.conns.get(&id)).map(|conn| conn.outbound.clone()) else {
            self.pending_clipboard = Some(text);
            return;
        };
        let transfer_id = self.next_transfer_id;
        self.next_transfer_id += 1;
        self.clipboard_activity = ClipboardActivity::Sending;
        self.last_skip = None;
        self.dirty = true;
        let events = self.events_tx.clone();
        tokio::spawn(async move {
            let bytes = text.into_bytes();
            let begin = Message::ClipboardBegin {
                transfer_id,
                format: ClipboardFormat::PlainText,
                total_len: bytes.len() as u32,
                sha256: sha256(&bytes),
            };
            let mut ok = outbound.send_low(begin).await.is_ok();
            if ok {
                for (start, end) in chunk_offsets(bytes.len()) {
                    let chunk = Message::ClipboardChunk { transfer_id, offset: start as u32, data: bytes[start..end].to_vec() };
                    if outbound.send_low(chunk).await.is_err() {
                        ok = false;
                        break;
                    }
                }
            }
            let _ = events.send(Event::ClipboardSent { ok });
        });
    }

    pub fn on_clipboard_sent(&mut self, ok: bool) {
        self.clipboard_activity = ClipboardActivity::Idle;
        if ok {
            self.stats.clipboard_sent += 1;
            self.last_transfer_ms = Some(now_ms());
            self.last_direction = Some(TransferDirection::Outgoing);
        } else {
            self.clipboard_sync.forget_sent();
        }
        self.dirty = true;
    }

    pub fn on_clipboard_begin(&mut self, transfer_id: u64, total_len: u32, sha256: [u8; 32]) {
        if !self.config.settings.clipboard_sync {
            return;
        }
        match self.assembler.begin(transfer_id, total_len, sha256) {
            Ok(()) => {
                self.clipboard_activity = ClipboardActivity::Receiving;
                self.dirty = true;
            }
            Err(err) => tracing::warn!(error = %err, "rejected incoming clipboard transfer"),
        }
    }

    pub fn on_clipboard_chunk(&mut self, transfer_id: u64, offset: u32, data: &[u8]) {
        if !self.config.settings.clipboard_sync {
            return;
        }
        match self.assembler.chunk(transfer_id, offset, data) {
            Ok(None) => {}
            Ok(Some(assembled)) => {
                self.clipboard_activity = ClipboardActivity::Idle;
                self.dirty = true;
                if self.clipboard_sync.on_remote_text(assembled.hash) == RemoteDecision::Apply {
                    let platform = self.platform.clone();
                    let events = self.events_tx.clone();
                    let text = assembled.text;
                    tokio::task::spawn_blocking(move || {
                        let result = platform.write_clipboard(&text).map_err(|err| err.to_string());
                        let _ = events.send(Event::ClipboardWritten { result });
                    });
                }
            }
            Err(AssemblyError::UnknownTransfer) => {}
            Err(err) => {
                tracing::warn!(error = %err, "incoming clipboard transfer discarded");
                self.clipboard_activity = ClipboardActivity::Idle;
                self.dirty = true;
            }
        }
    }

    pub fn on_clipboard_written(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => {
                self.stats.clipboard_received += 1;
                self.last_transfer_ms = Some(now_ms());
                self.last_direction = Some(TransferDirection::Incoming);
            }
            Err(err) => {
                tracing::warn!(error = %err, "clipboard write failed");
                self.clipboard_sync.forget_sent();
                self.set_error("clipboard_write_failed");
            }
        }
        self.dirty = true;
    }
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    match std::str::from_utf8(bytes) {
        Ok(text) => text_hash(text),
        Err(_) => [0; 32],
    }
}
