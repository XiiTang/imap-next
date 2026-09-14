use super::*;
use imap_codec::encode::StreamedAppend;

pub(super) struct AppendState {
    pub handle: CommandHandle,
    pub spec: StreamedAppend,
    prefix: VecDeque<Fragment>,
    phase: Phase,
    written: u64,
}
enum Phase {
    Prefix(Option<Vec<u8>>),
    PrefixWritten(Option<Vec<u8>>),
    MetadataContinuation(Vec<u8>),
    HeaderContinuation,
    Ready(bool),
    Chunk(Vec<u8>),
    ChunkWritten(usize),
    Finish,
    Finished,
}
impl AppendState {
    pub fn new(handle: CommandHandle, spec: StreamedAppend) -> Result<Self, &'static str> {
        let prefix = spec.encode_prefix()?.collect();
        Ok(Self {
            handle,
            spec,
            prefix,
            phase: Phase::Prefix(None),
            written: 0,
        })
    }
    pub fn continuation(&mut self) -> bool {
        match std::mem::replace(&mut self.phase, Phase::Finished) {
            Phase::MetadataContinuation(bytes) => {
                self.phase = Phase::Prefix(Some(bytes));
                true
            }
            Phase::HeaderContinuation => {
                self.phase = Phase::Ready(false);
                true
            }
            phase => {
                self.phase = phase;
                false
            }
        }
    }
    pub fn check_chunk(&self, bytes: &[u8]) -> Result<(), &'static str> {
        if !matches!(self.phase, Phase::Ready(true)) {
            return Err("APPEND is not ready for a chunk");
        }
        if bytes.is_empty() || bytes.len() > 65536 {
            return Err("APPEND chunks must contain 1 through 65536 bytes");
        }
        if bytes.len() as u64 > self.spec.length - self.written {
            return Err("APPEND chunk exceeds the declared remaining length");
        }
        if !self.spec.binary && bytes.contains(&0) {
            return Err("NUL requires a BINARY APPEND literal");
        }
        Ok(())
    }
    pub fn set_chunk(&mut self, bytes: Vec<u8>) -> Result<(), &'static str> {
        self.check_chunk(&bytes)?;
        self.phase = Phase::Chunk(bytes);
        Ok(())
    }
    pub fn check_finish(&self) -> Result<(), &'static str> {
        if !matches!(self.phase, Phase::Ready(true)) {
            return Err("APPEND is not ready to finish");
        }
        if self.written != self.spec.length {
            return Err("APPEND has not received the declared number of bytes");
        }
        Ok(())
    }
    pub fn finish(&mut self) -> Result<(), &'static str> {
        self.check_finish()?;
        self.phase = Phase::Finish;
        Ok(())
    }
    pub fn push_to_buffer(mut self, out: &mut Vec<u8>) -> Self {
        self.phase = match self.phase {
            Phase::Prefix(accepted) => {
                if let Some(bytes) = accepted {
                    out.extend(bytes);
                }
                let mut pending = None;
                while let Some(fragment) = self.prefix.pop_front() {
                    match fragment {
                        Fragment::Line { data }
                        | Fragment::Literal {
                            data,
                            mode: LiteralMode::NonSync,
                        } => out.extend(data),
                        Fragment::Literal {
                            data,
                            mode: LiteralMode::Sync,
                        } => {
                            pending = Some(data);
                            break;
                        }
                    }
                }
                Phase::PrefixWritten(pending)
            }
            Phase::Chunk(bytes) => {
                let count = bytes.len();
                out.extend(bytes);
                Phase::ChunkWritten(count)
            }
            Phase::Finish => {
                out.extend_from_slice(b"\r\n");
                Phase::Finished
            }
            phase => phase,
        };
        self
    }
    pub fn finish_sending(mut self) -> FinishSendingResult<Self> {
        let event = match self.phase {
            Phase::PrefixWritten(Some(bytes)) => {
                self.phase = Phase::MetadataContinuation(bytes);
                None
            }
            Phase::PrefixWritten(None) if self.spec.mode == LiteralMode::Sync => {
                self.phase = Phase::HeaderContinuation;
                None
            }
            Phase::PrefixWritten(None) | Phase::Ready(false) => {
                self.phase = Phase::Ready(true);
                Some(ClientSendEvent::AppendReady {
                    handle: self.handle,
                })
            }
            Phase::ChunkWritten(count) => {
                self.written += count as u64;
                self.phase = Phase::Ready(true);
                Some(ClientSendEvent::AppendChunkSent {
                    handle: self.handle,
                    written: self.written,
                })
            }
            Phase::Finished => {
                return FinishSendingResult::Completed {
                    event: ClientSendEvent::AppendSent {
                        handle: self.handle,
                    },
                }
            }
            _ => None,
        };
        FinishSendingResult::Uncompleted { state: self, event }
    }
}
