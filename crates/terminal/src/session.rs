/// PtySession wires together Pty + ScreenBuffer + VtParser + VtProcessor.
/// One session per terminal pane.
use anyhow::Result;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
    /// Hold the Pty so Drop closes it when session ends.
    _pty: Arc<parking_lot::Mutex<Pty>>,
}

impl PtySession {
    /// Spawn a shell and start background reader/writer tasks.
    /// `event_tx` receives `AppEvent::TerminalDirty` whenever the screen updates.
    pub async fn spawn(
        id: SessionId,
        config: PtyConfig,
        event_tx: tokio::sync::broadcast::Sender<AppEvent>,
    ) -> Result<Arc<parking_lot::Mutex<Self>>> {
        let cols = config.size.cols as usize;
        let rows = config.size.rows as usize;

        let mut pty = Pty::spawn(config).await?;

        // Take I/O halves before wrapping pty in Arc<Mutex>.
        let reader = pty.take_reader()
            .ok_or_else(|| anyhow::anyhow!("PTY reader already taken"))?;
        let writer = pty.take_writer()
            .ok_or_else(|| anyhow::anyhow!("PTY writer already taken"))?;

        // Channel for user input → PTY writer task.
        let (input_tx, mut input_rx) = mpsc::channel::<Vec<u8>>(64);

        let pty_arc = Arc::new(parking_lot::Mutex::new(pty));

        let session = Arc::new(parking_lot::Mutex::new(Self {
            id,
            screen: ScreenBuffer::new(cols, rows, SCROLLBACK),
            processor: VtProcessor::new(rows),
            parser: VtParser::new(),
            input_tx,
            _pty: pty_arc,
        }));

        // ── Writer task: input_rx → PTY ───────────────────────────────────────
        {
            let mut writer = writer;
            tokio::spawn(async move {
                while let Some(data) = input_rx.recv().await {
                    if let Err(e) = writer.write_all(&data).await {
                        warn!("PTY writer error: {e}");
                        break;
                    }
                    let _ = writer.flush().await;
                }
            });
        }

        // ── Reader task: PTY → screen buffer ─────────────────────────────────
        {
            let session_arc = Arc::clone(&session);
            let pane_id = core::ids::PaneId::new(id.0); // reuse numeric id as PaneId
            let mut reader = reader;
            tokio::spawn(async move {
                let mut buf = vec![0u8; READ_BUF];
                loop {
                    match reader.read(&mut buf).await {
                        Ok(0) => {
                            debug!("PTY reader EOF");
                            break;
                        }
                        Ok(n) => {
                            let bytes = &buf[..n];
                            {
                                let mut sess = session_arc.lock();
                                sess.ingest(bytes);
                            }
                            let _ = event_tx.send(AppEvent::TerminalDirty(pane_id));
                        }
                        Err(e) => {
                            warn!("PTY reader error: {e}");
                            break;
                        }
                    }
                }
            });
        }

        debug!("PtySession {id} spawned with reader/writer tasks");
        Ok(session)
    }

    /// Write user input (keystrokes) to the PTY.
    pub fn write_input(&self, data: Vec<u8>) {
        // Use try_send to avoid blocking; drop if channel full.
        if let Err(e) = self.input_tx.try_send(data) {
            warn!("PTY input channel error: {e}");
        }
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
