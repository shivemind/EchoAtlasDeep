/// Crossterm event reader → AppEvent bridge.
/// Runs as a dedicated tokio task; exits when the channel closes.
use crossterm::event::{
    Event, EventStream, KeyCode as CtKeyCode, KeyEventKind,
    KeyModifiers as CtMods, MouseButton as CtBtn, MouseEventKind as CtMouse,
};
use futures::StreamExt;
use tokio::sync::broadcast;
use tracing::warn;

use core::event::{
    AppEvent, KeyCode, KeyEvent, KeyModifiers, MouseButton,
    MouseEvent, MouseEventKind, TermSize,
};

/// Spawn the input task. Returns immediately; the task runs in the background.
pub fn spawn_input_task(tx: broadcast::Sender<AppEvent>) {
    tokio::spawn(async move {
        let mut stream = EventStream::new();
        loop {
            match stream.next().await {
                Some(Ok(event)) => {
                    if let Some(app_event) = translate_event(event) {
                        let quit = matches!(app_event, AppEvent::Quit);
                        let _ = tx.send(app_event);
                        if quit { break; }
                    }
                }
                Some(Err(e)) => warn!("Crossterm event error: {e}"),
                None => break,
            }
        }
    });
}

fn translate_event(ev: Event) -> Option<AppEvent> {
    match ev {
        Event::Key(k) if k.kind == KeyEventKind::Press || k.kind == KeyEventKind::Repeat => {
            Some(AppEvent::KeyInput(KeyEvent {
                code: translate_key(k.code)?,
                modifiers: translate_mods(k.modifiers),
            }))
        }
        Event::Mouse(m) => Some(AppEvent::MouseInput(MouseEvent {
            kind: translate_mouse_kind(m.kind)?,
            col: m.column,
            row: m.row,
            modifiers: translate_mods(m.modifiers),
        })),
        Event::Resize(cols, rows) => {
            Some(AppEvent::Resize(TermSize { cols, rows }))
        }
        Event::FocusGained => Some(AppEvent::FocusGained),
        Event::FocusLost   => Some(AppEvent::FocusLost),
        _ => None,
    }
}

fn translate_key(code: CtKeyCode) -> Option<KeyCode> {
    Some(match code {
        CtKeyCode::Char(c)  => KeyCode::Char(c),
        CtKeyCode::Enter    => KeyCode::Enter,
        CtKeyCode::Backspace => KeyCode::Backspace,
        CtKeyCode::Delete   => KeyCode::Delete,
        CtKeyCode::Esc      => KeyCode::Escape,
        CtKeyCode::Tab      => KeyCode::Tab,
        CtKeyCode::BackTab  => KeyCode::BackTab,
        CtKeyCode::Up       => KeyCode::Up,
        CtKeyCode::Down     => KeyCode::Down,
        CtKeyCode::Left     => KeyCode::Left,
        CtKeyCode::Right    => KeyCode::Right,
        CtKeyCode::Home     => KeyCode::Home,
        CtKeyCode::End      => KeyCode::End,
        CtKeyCode::PageUp   => KeyCode::PageUp,
        CtKeyCode::PageDown => KeyCode::PageDown,
        CtKeyCode::Insert   => KeyCode::Insert,
        CtKeyCode::F(n)     => KeyCode::F(n),
        CtKeyCode::Null     => KeyCode::Null,
        _                   => return None,
    })
}

fn translate_mods(mods: CtMods) -> KeyModifiers {
    let mut out = KeyModifiers::NONE;
    if mods.contains(CtMods::SHIFT)   { out |= KeyModifiers::SHIFT; }
    if mods.contains(CtMods::CONTROL) { out |= KeyModifiers::CONTROL; }
    if mods.contains(CtMods::ALT)     { out |= KeyModifiers::ALT; }
    if mods.contains(CtMods::SUPER)   { out |= KeyModifiers::SUPER; }
    out
}

fn translate_mouse_kind(kind: CtMouse) -> Option<MouseEventKind> {
    Some(match kind {
        CtMouse::Down(b)  => MouseEventKind::Down(translate_btn(b)),
        CtMouse::Up(b)    => MouseEventKind::Up(translate_btn(b)),
        CtMouse::Drag(b)  => MouseEventKind::Drag(translate_btn(b)),
        CtMouse::Moved    => MouseEventKind::Moved,
        CtMouse::ScrollDown => MouseEventKind::ScrollDown,
        CtMouse::ScrollUp   => MouseEventKind::ScrollUp,
        _ => return None,
    })
}

fn translate_btn(b: CtBtn) -> MouseButton {
    match b {
        CtBtn::Left   => MouseButton::Left,
        CtBtn::Right  => MouseButton::Right,
        CtBtn::Middle => MouseButton::Middle,
    }
}
