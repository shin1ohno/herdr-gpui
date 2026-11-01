//! Platform-independent shortcut typography; GPUI's Display omits Command on iOS.

pub(crate) fn glyphs(label: &str) -> String {
    label
        .split_whitespace()
        .enumerate()
        .map(|(index, key)| stroke(key, index == 0))
        .collect::<Vec<_>>()
        .join(" ")
}

fn stroke(mut key: &str, uppercase: bool) -> String {
    let mut modifiers = [false; 4];
    while let Some((modifier, rest)) = key.split_once('-') {
        let index = match modifier.to_ascii_lowercase().as_str() {
            "cmd" | "platform" | "super" | "win" => 0,
            "ctrl" => 1,
            "alt" => 2,
            "shift" => 3,
            _ => break,
        };
        modifiers[index] = true;
        key = rest;
    }
    let mut result = String::new();
    for (present, glyph) in modifiers.into_iter().zip(['⌘', '⌃', '⌥', '⇧']) {
        if present {
            result.push(glyph);
        }
    }
    let named = match key.to_ascii_lowercase().as_str() {
        "enter" => Some("↩"),
        "escape" => Some("⎋"),
        "tab" => Some("⇥"),
        "backspace" => Some("⌫"),
        "left" => Some("←"),
        "up" => Some("↑"),
        "down" => Some("↓"),
        "right" => Some("→"),
        "space" => Some("Space"), // i18n: data
        _ => None,
    };
    if let Some(named) = named {
        result.push_str(named);
    } else if uppercase || modifiers[3] {
        result.push_str(&key.to_uppercase());
    } else {
        result.push_str(key);
    }
    result
}
