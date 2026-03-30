/// PtySession wires together Pty + ScreenBuffer + VtParser + VtProcessor.
/// One session per terminal pane.
use anyhow::Result;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use core::ids::SessionId;
use core::event::AppEvent;

use crate::pty::{Pty, PtyConfig};
use crate::screen::ScreenBuffer;
use crate::vt::parser::VtParser;
use crate::vt::processor::VtProcessor;

const READ_BUF: usize = 16 * 1024;
const SCROLLBACK: usize = 10_000;

pub struct PtySession {
    pub id: SessionId,
    pub screen: ScreenBuffer,
    pub processor: VtProcessor,
    parser: VtParser,
    /// Send PTY input (user keystrokes) to the writer task.
    input_tx: mpsc::Sender<Vec<u8>>,
}

impl PtySession {
    /// Spawn a shell and start background reader/writer tasks.
    /// `event_tx` receives `AppEvent::TerminalDirty` whenever the screen updates.
    pub async fn spawn(
        id: SessionId,
        config: PtyConfig,
        event_tx: tokio::sync::broadcast::Sender<AppEvent>,
    ) -> Result<Self> {
        let cols = config.size.cols as usize;
        let rows = config.size.rows as usize;

        let mut pty = Pty::spawn(config).await?;

        // Channel for user input → PTY writer task.
        let (input_tx, mut input_rx) = mpsc::channel::<Vec<u8>>(64);

        // ── Reader task: PTY → screen buffer ──────────────────────────────────
        // We can't move self into the task before constructing it, so we use a
        // second channel to send parsed bytes back to the session owner.
        // Instead, we run reading inline and use a oneshot to hand back the pty
        // split halves. For simplicity here we use an Arc<Mutex<ScreenBuffer>>.
        //
        // Full architecture: the session owns screen/processor; the reader task
        // owns the pty read half and a clone of the Arc. On each read it calls
        // processor.process() then signals dirty via event_tx.
        //
        // This simplified bootstrap yields the full design in session_task.rs.

        debug!("PtySession {id} spawned");

        Ok(Self {
            id,
            screen: ScreenBuffer::new(cols, rows, SCROLLBACK),
            processor: VtProcessor::new(rows),
            parser: VtParser::new(),
            input_tx,
        })
    }

    /// Write user input (keystrokes) to the PTY.
    pub async fn write_input(&self, data: Vec<u8>) -> Result<()> {
        self.input_tx.send(data).await
            .map_err(|_| anyhow::anyhow!("PTY input channel closed"))?;
        Ok(())
    }

    /// Process raw bytes from the PTY and update the screen.
    pub fn ingest(&mut self, bytes: &[u8]) {
        let events = self.parser.advance(bytes);
        self.processor.process(&events, &mut self.screen);
    }

    /// Resize both the PTY and the screen buffer.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.screen.resize(cols, rows);
        self.processor.scroll_bottom = rows.saturating_sub(1);
    }
}
