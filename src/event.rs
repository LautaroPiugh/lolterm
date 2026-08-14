use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use crate::config::Chord;

pub fn is_quit_key(key: KeyEvent, chord: Chord) -> bool {
    chord.matches(key)
}

pub fn is_prefix_key(key: KeyEvent, chord: Chord) -> bool {
    chord.matches(key)
}

pub fn encode_key(key: KeyEvent) -> Option<Vec<u8>> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    let mut bytes = match key.code {
        KeyCode::Char(ch) if ctrl => encode_ctrl_char(ch),
        KeyCode::Char(ch) => {
            let mut buf = [0u8; 4];
            ch.encode_utf8(&mut buf).as_bytes().to_vec()
        }
        KeyCode::Enter => b"\r".to_vec(),
        KeyCode::Tab => b"\t".to_vec(),
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Left => encode_csi(b'D', key.modifiers),
        KeyCode::Right => encode_csi(b'C', key.modifiers),
        KeyCode::Up => encode_csi(b'A', key.modifiers),
        KeyCode::Down => encode_csi(b'B', key.modifiers),
        KeyCode::Home => encode_csi(b'H', key.modifiers),
        KeyCode::End => encode_csi(b'F', key.modifiers),
        KeyCode::PageUp => encode_tilde("5", key.modifiers),
        KeyCode::PageDown => encode_tilde("6", key.modifiers),
        KeyCode::Delete => encode_tilde("3", key.modifiers),
        KeyCode::Insert => encode_tilde("2", key.modifiers),
        KeyCode::F(n) => encode_fn_key(n)?,
        _ => return None,
    };

    if alt && matches!(key.code, KeyCode::Char(_)) {
        bytes.insert(0, 0x1b);
    }

    Some(bytes)
}

fn encode_ctrl_char(ch: char) -> Vec<u8> {
    let lower = ch.to_ascii_lowercase();
    if lower.is_ascii_alphabetic() {
        vec![lower as u8 & 0x1f]
    } else if ch == ' ' {
        vec![0]
    } else {
        let mut buf = [0u8; 4];
        ch.encode_utf8(&mut buf).as_bytes().to_vec()
    }
}

fn csi_modifier(modifiers: KeyModifiers) -> u8 {
    let mut value = 1;
    if modifiers.contains(KeyModifiers::SHIFT) {
        value += 1;
    }
    if modifiers.contains(KeyModifiers::ALT) {
        value += 2;
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        value += 4;
    }
    value
}

fn encode_csi(letter: u8, modifiers: KeyModifiers) -> Vec<u8> {
    let modifier = csi_modifier(modifiers);
    if modifier == 1 {
        vec![0x1b, b'[', letter]
    } else {
        format!("\x1b[1;{modifier}{}", letter as char).into_bytes()
    }
}

fn encode_tilde(code: &str, modifiers: KeyModifiers) -> Vec<u8> {
    let modifier = csi_modifier(modifiers);
    if modifier == 1 {
        format!("\x1b[{code}~").into_bytes()
    } else {
        format!("\x1b[{code};{modifier}~").into_bytes()
    }
}

fn encode_fn_key(n: u8) -> Option<Vec<u8>> {
    Some(match n {
        1 => b"\x1bOP".to_vec(),
        2 => b"\x1bOQ".to_vec(),
        3 => b"\x1bOR".to_vec(),
        4 => b"\x1bOS".to_vec(),
        5 => b"\x1b[15~".to_vec(),
        6 => b"\x1b[17~".to_vec(),
        7 => b"\x1b[18~".to_vec(),
        8 => b"\x1b[19~".to_vec(),
        9 => b"\x1b[20~".to_vec(),
        10 => b"\x1b[21~".to_vec(),
        11 => b"\x1b[23~".to_vec(),
        12 => b"\x1b[24~".to_vec(),
        _ => return None,
    })
}

pub fn encode_mouse(event: MouseEvent, col: u16, row: u16) -> Option<Vec<u8>> {
    let (button, release) = match event.kind {
        MouseEventKind::Down(MouseButton::Left) => (0, false),
        MouseEventKind::Down(MouseButton::Middle) => (1, false),
        MouseEventKind::Down(MouseButton::Right) => (2, false),
        MouseEventKind::Up(MouseButton::Left) => (0, true),
        MouseEventKind::Up(MouseButton::Middle) => (1, true),
        MouseEventKind::Up(MouseButton::Right) => (2, true),
        MouseEventKind::Drag(MouseButton::Left) => (32, false),
        MouseEventKind::Drag(MouseButton::Middle) => (33, false),
        MouseEventKind::Drag(MouseButton::Right) => (34, false),
        MouseEventKind::ScrollUp => (64, false),
        MouseEventKind::ScrollDown => (65, false),
        MouseEventKind::ScrollLeft => (66, false),
        MouseEventKind::ScrollRight => (67, false),
        _ => return None,
    };

    let mut button = button;
    if event.modifiers.contains(KeyModifiers::SHIFT) {
        button += 4;
    }
    if event.modifiers.contains(KeyModifiers::ALT) {
        button += 8;
    }
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        button += 16;
    }

    let suffix = if release { b'm' } else { b'M' };
    Some(format!("\x1b[<{button};{col};{row}{}", suffix as char).into_bytes())
}
