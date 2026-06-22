use rdev::Button;
use rdev::Key;

pub fn key_to_string(key: &Key) -> Option<String> {
    match key {
        Key::Alt => Some("alt_l".to_string()),
        Key::AltGr => Some("alt_r".to_string()),
        Key::Backspace => Some("backspace".to_string()),
        Key::CapsLock => Some("caps_lock".to_string()),
        Key::ControlLeft => Some("ctrl_l".to_string()),
        Key::ControlRight => Some("ctrl_r".to_string()),
        Key::Delete => Some("delete".to_string()),
        Key::DownArrow => Some("down".to_string()),
        Key::End => Some("end".to_string()),
        Key::Escape => Some("esc".to_string()),
        Key::F1 => Some("f1".to_string()),
        Key::F2 => Some("f2".to_string()),
        Key::F3 => Some("f3".to_string()),
        Key::F4 => Some("f4".to_string()),
        Key::F5 => Some("f5".to_string()),
        Key::F6 => Some("f6".to_string()),
        Key::F7 => Some("f7".to_string()),
        Key::F8 => Some("f8".to_string()),
        Key::F9 => Some("f9".to_string()),
        Key::F10 => Some("f10".to_string()),
        Key::F11 => Some("f11".to_string()),
        Key::F12 => Some("f12".to_string()),
        Key::Home => Some("home".to_string()),
        Key::Insert => Some("insert".to_string()),
        Key::LeftArrow => Some("left".to_string()),
        Key::MetaLeft => Some("meta".to_string()),
        Key::MetaRight => Some("meta_r".to_string()),
        Key::NumLock => Some("numpad_lock".to_string()),
        Key::PageDown => Some("page_down".to_string()),
        Key::PageUp => Some("page_up".to_string()),
        Key::Pause => Some("pause".to_string()),
        Key::PrintScreen => Some("print_screen".to_string()),
        Key::Return => Some("enter".to_string()),
        Key::RightArrow => Some("right".to_string()),
        Key::ScrollLock => Some("scroll_lock".to_string()),
        Key::ShiftLeft => Some("shift_l".to_string()),
        Key::ShiftRight => Some("shift_r".to_string()),
        Key::Space => Some("space".to_string()),
        Key::Tab => Some("tab".to_string()),
        Key::UpArrow => Some("up".to_string()),

        // Letter keys
        Key::KeyA => Some("a".to_string()),
        Key::KeyB => Some("b".to_string()),
        Key::KeyC => Some("c".to_string()),
        Key::KeyD => Some("d".to_string()),
        Key::KeyE => Some("e".to_string()),
        Key::KeyF => Some("f".to_string()),
        Key::KeyG => Some("g".to_string()),
        Key::KeyH => Some("h".to_string()),
        Key::KeyI => Some("i".to_string()),
        Key::KeyJ => Some("j".to_string()),
        Key::KeyK => Some("k".to_string()),
        Key::KeyL => Some("l".to_string()),
        Key::KeyM => Some("m".to_string()),
        Key::KeyN => Some("n".to_string()),
        Key::KeyO => Some("o".to_string()),
        Key::KeyP => Some("p".to_string()),
        Key::KeyQ => Some("q".to_string()),
        Key::KeyR => Some("r".to_string()),
        Key::KeyS => Some("s".to_string()),
        Key::KeyT => Some("t".to_string()),
        Key::KeyU => Some("u".to_string()),
        Key::KeyV => Some("v".to_string()),
        Key::KeyW => Some("w".to_string()),
        Key::KeyX => Some("x".to_string()),
        Key::KeyY => Some("y".to_string()),
        Key::KeyZ => Some("z".to_string()),

        // Number row
        Key::Num0 => Some("0".to_string()),
        Key::Num1 => Some("1".to_string()),
        Key::Num2 => Some("2".to_string()),
        Key::Num3 => Some("3".to_string()),
        Key::Num4 => Some("4".to_string()),
        Key::Num5 => Some("5".to_string()),
        Key::Num6 => Some("6".to_string()),
        Key::Num7 => Some("7".to_string()),
        Key::Num8 => Some("8".to_string()),
        Key::Num9 => Some("9".to_string()),

        // Symbol keys
        Key::BackQuote => Some("`".to_string()),
        Key::Minus => Some("-".to_string()),
        Key::Equal => Some("=".to_string()),
        Key::LeftBracket => Some("[".to_string()),
        Key::RightBracket => Some("]".to_string()),
        Key::SemiColon => Some(";".to_string()),
        Key::Quote => Some("'".to_string()),
        Key::BackSlash => Some("backslash".to_string()),
        Key::IntlBackslash => Some("intl_backslash".to_string()),
        Key::Comma => Some(",".to_string()),
        Key::Dot => Some(".".to_string()),
        Key::Slash => Some("/".to_string()),

        // Numpad
        Key::Kp0 => Some("numpad0".to_string()),
        Key::Kp1 => Some("numpad1".to_string()),
        Key::Kp2 => Some("numpad2".to_string()),
        Key::Kp3 => Some("numpad3".to_string()),
        Key::Kp4 => Some("numpad4".to_string()),
        Key::Kp5 => Some("numpad5".to_string()),
        Key::Kp6 => Some("numpad6".to_string()),
        Key::Kp7 => Some("numpad7".to_string()),
        Key::Kp8 => Some("numpad8".to_string()),
        Key::Kp9 => Some("numpad9".to_string()),
        Key::KpReturn => Some("numpad_enter".to_string()),
        Key::KpMinus => Some("numpad_subtract".to_string()),
        Key::KpPlus => Some("numpad_add".to_string()),
        Key::KpMultiply => Some("numpad_multiply".to_string()),
        Key::KpDivide => Some("numpad_divide".to_string()),
        Key::KpDelete => Some("numpad_decimal".to_string()),

        Key::Function => Some("fn".to_string()),

        Key::Unknown(vk) => vk_to_string(*vk).map(|s| s.to_string()),
    }
}

pub fn button_to_string(button: &Button) -> Option<String> {
    match button {
        Button::Left => Some("mouse_left".to_string()),
        Button::Right => Some("mouse_right".to_string()),
        Button::Middle => Some("mouse_middle".to_string()),
        Button::Unknown(1) => Some("mouse_x1".to_string()),
        Button::Unknown(2) => Some("mouse_x2".to_string()),
        Button::Unknown(vk) => Some(format!("mouse_unknown_{}", vk)),
    }
}

fn vk_to_string(vk: u32) -> Option<&'static str> {
    match vk {
        48 => Some("0"),
        49 => Some("1"),
        50 => Some("2"),
        51 => Some("3"),
        52 => Some("4"),
        53 => Some("5"),
        54 => Some("6"),
        55 => Some("7"),
        56 => Some("8"),
        57 => Some("9"),
        96 => Some("numpad0"),
        97 => Some("numpad1"),
        98 => Some("numpad2"),
        99 => Some("numpad3"),
        100 => Some("numpad4"),
        101 => Some("numpad5"),
        102 => Some("numpad6"),
        103 => Some("numpad7"),
        104 => Some("numpad8"),
        105 => Some("numpad9"),
        106 => Some("numpad_multiply"),
        107 => Some("numpad_add"),
        109 => Some("numpad_subtract"),
        110 => Some("numpad_decimal"),
        111 => Some("numpad_divide"),
        144 => Some("numpad_lock"),
        65 => Some("a"),
        66 => Some("b"),
        67 => Some("c"),
        68 => Some("d"),
        69 => Some("e"),
        70 => Some("f"),
        71 => Some("g"),
        72 => Some("h"),
        73 => Some("i"),
        74 => Some("j"),
        75 => Some("k"),
        76 => Some("l"),
        77 => Some("m"),
        78 => Some("n"),
        79 => Some("o"),
        80 => Some("p"),
        81 => Some("q"),
        82 => Some("r"),
        83 => Some("s"),
        84 => Some("t"),
        85 => Some("u"),
        86 => Some("v"),
        87 => Some("w"),
        88 => Some("x"),
        89 => Some("y"),
        90 => Some("z"),
        91 => Some("meta"),
        19 => Some("pause"),
        34 => Some("page_down"),
        33 => Some("page_up"),
        35 => Some("end"),
        36 => Some("home"),
        237 => Some("fn"),
        8 => Some("backspace"),
        9 => Some("tab"),
        13 => Some("enter"),
        20 => Some("caps_lock"),
        27 => Some("esc"),
        32 => Some("space"),
        16 => Some("shift"),
        160 => Some("shift_l"),
        161 => Some("shift_r"),
        17 => Some("ctrl"),
        162 => Some("ctrl_l"),
        163 => Some("ctrl_r"),
        18 => Some("alt"),
        164 => Some("alt_l"),
        165 => Some("alt_r"),
        93 => Some("menu"),
        186 => Some(";"),
        187 => Some("="),
        188 => Some(","),
        189 => Some("-"),
        190 => Some("."),
        191 => Some("/"),
        192 => Some("`"),
        219 => Some("["),
        220 => Some("backslash"),
        221 => Some("]"),
        222 => Some("'"),
        112 => Some("f1"),
        113 => Some("f2"),
        114 => Some("f3"),
        115 => Some("f4"),
        116 => Some("f5"),
        117 => Some("f6"),
        118 => Some("f7"),
        119 => Some("f8"),
        120 => Some("f9"),
        121 => Some("f10"),
        122 => Some("f11"),
        123 => Some("f12"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── key_to_string ──────────────────────────────────────────

    #[test]
    fn test_modifier_keys() {
        assert_eq!(key_to_string(&Key::Alt), Some("alt_l".into()));
        assert_eq!(key_to_string(&Key::AltGr), Some("alt_r".into()));
        assert_eq!(key_to_string(&Key::ControlLeft), Some("ctrl_l".into()));
        assert_eq!(key_to_string(&Key::ControlRight), Some("ctrl_r".into()));
        assert_eq!(key_to_string(&Key::ShiftLeft), Some("shift_l".into()));
        assert_eq!(key_to_string(&Key::ShiftRight), Some("shift_r".into()));
        assert_eq!(key_to_string(&Key::MetaLeft), Some("meta".into()));
        assert_eq!(key_to_string(&Key::MetaRight), Some("meta_r".into()));
    }

    #[test]
    fn test_navigation_keys() {
        assert_eq!(key_to_string(&Key::UpArrow), Some("up".into()));
        assert_eq!(key_to_string(&Key::DownArrow), Some("down".into()));
        assert_eq!(key_to_string(&Key::LeftArrow), Some("left".into()));
        assert_eq!(key_to_string(&Key::RightArrow), Some("right".into()));
        assert_eq!(key_to_string(&Key::Home), Some("home".into()));
        assert_eq!(key_to_string(&Key::End), Some("end".into()));
        assert_eq!(key_to_string(&Key::PageUp), Some("page_up".into()));
        assert_eq!(key_to_string(&Key::PageDown), Some("page_down".into()));
        assert_eq!(key_to_string(&Key::Insert), Some("insert".into()));
        assert_eq!(key_to_string(&Key::Delete), Some("delete".into()));
    }

    #[test]
    fn test_editing_keys() {
        assert_eq!(key_to_string(&Key::Backspace), Some("backspace".into()));
        assert_eq!(key_to_string(&Key::Return), Some("enter".into()));
        assert_eq!(key_to_string(&Key::Tab), Some("tab".into()));
        assert_eq!(key_to_string(&Key::Space), Some("space".into()));
        assert_eq!(key_to_string(&Key::CapsLock), Some("caps_lock".into()));
        assert_eq!(key_to_string(&Key::Escape), Some("esc".into()));
    }

    #[test]
    fn test_function_keys() {
        assert_eq!(key_to_string(&Key::F1), Some("f1".into()));
        assert_eq!(key_to_string(&Key::F2), Some("f2".into()));
        assert_eq!(key_to_string(&Key::F3), Some("f3".into()));
        assert_eq!(key_to_string(&Key::F4), Some("f4".into()));
        assert_eq!(key_to_string(&Key::F5), Some("f5".into()));
        assert_eq!(key_to_string(&Key::F6), Some("f6".into()));
        assert_eq!(key_to_string(&Key::F7), Some("f7".into()));
        assert_eq!(key_to_string(&Key::F8), Some("f8".into()));
        assert_eq!(key_to_string(&Key::F9), Some("f9".into()));
        assert_eq!(key_to_string(&Key::F10), Some("f10".into()));
        assert_eq!(key_to_string(&Key::F11), Some("f11".into()));
        assert_eq!(key_to_string(&Key::F12), Some("f12".into()));
    }

    #[test]
    fn test_system_keys() {
        assert_eq!(key_to_string(&Key::PrintScreen), Some("print_screen".into()));
        assert_eq!(key_to_string(&Key::ScrollLock), Some("scroll_lock".into()));
        assert_eq!(key_to_string(&Key::Pause), Some("pause".into()));
        assert_eq!(key_to_string(&Key::NumLock), Some("numpad_lock".into()));
        assert_eq!(key_to_string(&Key::Function), Some("fn".into()));
    }

    #[test]
    fn test_letter_keys() {
        assert_eq!(key_to_string(&Key::KeyA), Some("a".into()));
        assert_eq!(key_to_string(&Key::KeyZ), Some("z".into()));
        assert_eq!(key_to_string(&Key::KeyM), Some("m".into()));
    }

    #[test]
    fn test_number_row() {
        assert_eq!(key_to_string(&Key::Num0), Some("0".into()));
        assert_eq!(key_to_string(&Key::Num9), Some("9".into()));
    }

    #[test]
    fn test_symbol_keys() {
        assert_eq!(key_to_string(&Key::BackQuote), Some("`".into()));
        assert_eq!(key_to_string(&Key::Minus), Some("-".into()));
        assert_eq!(key_to_string(&Key::Equal), Some("=".into()));
        assert_eq!(key_to_string(&Key::LeftBracket), Some("[".into()));
        assert_eq!(key_to_string(&Key::RightBracket), Some("]".into()));
        assert_eq!(key_to_string(&Key::SemiColon), Some(";".into()));
        assert_eq!(key_to_string(&Key::Quote), Some("'".into()));
        assert_eq!(key_to_string(&Key::BackSlash), Some("backslash".into()));
        assert_eq!(key_to_string(&Key::IntlBackslash), Some("intl_backslash".into()));
        assert_eq!(key_to_string(&Key::Comma), Some(",".into()));
        assert_eq!(key_to_string(&Key::Dot), Some(".".into()));
        assert_eq!(key_to_string(&Key::Slash), Some("/".into()));
    }

    #[test]
    fn test_numpad_keys() {
        assert_eq!(key_to_string(&Key::Kp0), Some("numpad0".into()));
        assert_eq!(key_to_string(&Key::Kp9), Some("numpad9".into()));
        assert_eq!(key_to_string(&Key::KpReturn), Some("numpad_enter".into()));
        assert_eq!(key_to_string(&Key::KpMinus), Some("numpad_subtract".into()));
        assert_eq!(key_to_string(&Key::KpPlus), Some("numpad_add".into()));
        assert_eq!(key_to_string(&Key::KpMultiply), Some("numpad_multiply".into()));
        assert_eq!(key_to_string(&Key::KpDivide), Some("numpad_divide".into()));
        assert_eq!(key_to_string(&Key::KpDelete), Some("numpad_decimal".into()));
    }

    #[test]
    fn test_unknown_key_with_known_vk() {
        assert_eq!(key_to_string(&Key::Unknown(48)), Some("0".into()));
        assert_eq!(key_to_string(&Key::Unknown(65)), Some("a".into()));
        assert_eq!(key_to_string(&Key::Unknown(112)), Some("f1".into()));
    }

    #[test]
    fn test_unknown_key_with_unmapped_vk() {
        assert_eq!(key_to_string(&Key::Unknown(999)), None);
        assert_eq!(key_to_string(&Key::Unknown(0)), None);
    }

    // ── button_to_string ───────────────────────────────────────

    #[test]
    fn test_standard_mouse_buttons() {
        assert_eq!(button_to_string(&Button::Left), Some("mouse_left".into()));
        assert_eq!(button_to_string(&Button::Right), Some("mouse_right".into()));
        assert_eq!(button_to_string(&Button::Middle), Some("mouse_middle".into()));
    }

    #[test]
    fn test_side_buttons_x1_x2() {
        assert_eq!(button_to_string(&Button::Unknown(1)), Some("mouse_x1".into()));
        assert_eq!(button_to_string(&Button::Unknown(2)), Some("mouse_x2".into()));
    }

    #[test]
    fn test_unknown_mouse_buttons() {
        assert_eq!(button_to_string(&Button::Unknown(3)), Some("mouse_unknown_3".into()));
        assert_eq!(button_to_string(&Button::Unknown(99)), Some("mouse_unknown_99".into()));
    }
}
