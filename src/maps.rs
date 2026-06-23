use std::borrow::Cow;

use rdev::Button;
use rdev::Key;

macro_rules! borrowed {
    ($s:expr) => {
        Some(Cow::Borrowed($s))
    };
}

pub fn key_to_string(key: &Key) -> Option<Cow<'static, str>> {
    match key {
        Key::Alt => borrowed!("alt_l"),
        Key::AltGr => borrowed!("alt_r"),
        Key::Backspace => borrowed!("backspace"),
        Key::CapsLock => borrowed!("caps_lock"),
        Key::ControlLeft => borrowed!("ctrl_l"),
        Key::ControlRight => borrowed!("ctrl_r"),
        Key::Delete => borrowed!("delete"),
        Key::DownArrow => borrowed!("down"),
        Key::End => borrowed!("end"),
        Key::Escape => borrowed!("esc"),
        Key::F1 => borrowed!("f1"),
        Key::F2 => borrowed!("f2"),
        Key::F3 => borrowed!("f3"),
        Key::F4 => borrowed!("f4"),
        Key::F5 => borrowed!("f5"),
        Key::F6 => borrowed!("f6"),
        Key::F7 => borrowed!("f7"),
        Key::F8 => borrowed!("f8"),
        Key::F9 => borrowed!("f9"),
        Key::F10 => borrowed!("f10"),
        Key::F11 => borrowed!("f11"),
        Key::F12 => borrowed!("f12"),
        Key::Home => borrowed!("home"),
        Key::Insert => borrowed!("insert"),
        Key::LeftArrow => borrowed!("left"),
        Key::MetaLeft => borrowed!("meta"),
        Key::MetaRight => borrowed!("meta_r"),
        Key::NumLock => borrowed!("numpad_lock"),
        Key::PageDown => borrowed!("page_down"),
        Key::PageUp => borrowed!("page_up"),
        Key::Pause => borrowed!("pause"),
        Key::PrintScreen => borrowed!("print_screen"),
        Key::Return => borrowed!("enter"),
        Key::RightArrow => borrowed!("right"),
        Key::ScrollLock => borrowed!("scroll_lock"),
        Key::ShiftLeft => borrowed!("shift_l"),
        Key::ShiftRight => borrowed!("shift_r"),
        Key::Space => borrowed!("space"),
        Key::Tab => borrowed!("tab"),
        Key::UpArrow => borrowed!("up"),

        // Letter keys
        Key::KeyA => borrowed!("a"),
        Key::KeyB => borrowed!("b"),
        Key::KeyC => borrowed!("c"),
        Key::KeyD => borrowed!("d"),
        Key::KeyE => borrowed!("e"),
        Key::KeyF => borrowed!("f"),
        Key::KeyG => borrowed!("g"),
        Key::KeyH => borrowed!("h"),
        Key::KeyI => borrowed!("i"),
        Key::KeyJ => borrowed!("j"),
        Key::KeyK => borrowed!("k"),
        Key::KeyL => borrowed!("l"),
        Key::KeyM => borrowed!("m"),
        Key::KeyN => borrowed!("n"),
        Key::KeyO => borrowed!("o"),
        Key::KeyP => borrowed!("p"),
        Key::KeyQ => borrowed!("q"),
        Key::KeyR => borrowed!("r"),
        Key::KeyS => borrowed!("s"),
        Key::KeyT => borrowed!("t"),
        Key::KeyU => borrowed!("u"),
        Key::KeyV => borrowed!("v"),
        Key::KeyW => borrowed!("w"),
        Key::KeyX => borrowed!("x"),
        Key::KeyY => borrowed!("y"),
        Key::KeyZ => borrowed!("z"),

        // Number row
        Key::Num0 => borrowed!("0"),
        Key::Num1 => borrowed!("1"),
        Key::Num2 => borrowed!("2"),
        Key::Num3 => borrowed!("3"),
        Key::Num4 => borrowed!("4"),
        Key::Num5 => borrowed!("5"),
        Key::Num6 => borrowed!("6"),
        Key::Num7 => borrowed!("7"),
        Key::Num8 => borrowed!("8"),
        Key::Num9 => borrowed!("9"),

        // Symbol keys
        Key::BackQuote => borrowed!("`"),
        Key::Minus => borrowed!("-"),
        Key::Equal => borrowed!("="),
        Key::LeftBracket => borrowed!("["),
        Key::RightBracket => borrowed!("]"),
        Key::SemiColon => borrowed!(";"),
        Key::Quote => borrowed!("'"),
        Key::BackSlash => borrowed!("backslash"),
        Key::IntlBackslash => borrowed!("intl_backslash"),
        Key::Comma => borrowed!(","),
        Key::Dot => borrowed!("."),
        Key::Slash => borrowed!("/"),

        // Numpad
        Key::Kp0 => borrowed!("numpad0"),
        Key::Kp1 => borrowed!("numpad1"),
        Key::Kp2 => borrowed!("numpad2"),
        Key::Kp3 => borrowed!("numpad3"),
        Key::Kp4 => borrowed!("numpad4"),
        Key::Kp5 => borrowed!("numpad5"),
        Key::Kp6 => borrowed!("numpad6"),
        Key::Kp7 => borrowed!("numpad7"),
        Key::Kp8 => borrowed!("numpad8"),
        Key::Kp9 => borrowed!("numpad9"),
        Key::KpReturn => borrowed!("numpad_enter"),
        Key::KpMinus => borrowed!("numpad_subtract"),
        Key::KpPlus => borrowed!("numpad_add"),
        Key::KpMultiply => borrowed!("numpad_multiply"),
        Key::KpDivide => borrowed!("numpad_divide"),
        Key::KpDelete => borrowed!("numpad_decimal"),

        Key::Function => borrowed!("fn"),

        Key::Unknown(vk) => vk_to_string(*vk).map(Cow::Borrowed),
    }
}

pub fn button_to_string(button: &Button) -> Option<Cow<'static, str>> {
    match button {
        Button::Left => borrowed!("mouse_left"),
        Button::Right => borrowed!("mouse_right"),
        Button::Middle => borrowed!("mouse_middle"),
        Button::Unknown(1) => borrowed!("mouse_x1"),
        Button::Unknown(2) => borrowed!("mouse_x2"),
        Button::Unknown(vk) => Some(Cow::Owned(format!("mouse_unknown_{}", vk))),
    }
}

pub fn vk_to_string(vk: u32) -> Option<&'static str> {
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
