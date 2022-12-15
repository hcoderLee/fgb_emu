use std::fmt::{Display, Formatter};
use crate::core::joypad::{JoypadKey};

/// Gameboy buttons
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum GbBtn {
    LEFT = 0x01,
    UP = 0x02,
    RIGHT = 0x04,
    DOWN = 0x08,
    A = 0x10,
    B = 0x20,
    START = 0x40,
    SELECT = 0x80,
}

impl Display for GbBtn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let btn_name = match self {
            GbBtn::LEFT => "Left",
            GbBtn::UP => "Up",
            GbBtn::RIGHT => "Right",
            GbBtn::DOWN => "Down",
            GbBtn::A => "A",
            GbBtn::B => "B",
            GbBtn::START => "Start",
            GbBtn::SELECT => "Select"
        };
        write!(f, "{}", btn_name)
    }
}

/// 键盘按键和game boy按键的映射
pub const KEY_MAPS: [(GbBtn, JoypadKey); 8] = [
    (GbBtn::RIGHT, JoypadKey::Right),
    (GbBtn::UP, JoypadKey::Up),
    (GbBtn::LEFT, JoypadKey::Left),
    (GbBtn::DOWN, JoypadKey::Down),
    (GbBtn::A, JoypadKey::A),
    (GbBtn::B, JoypadKey::B),
    (GbBtn::SELECT, JoypadKey::Select),
    (GbBtn::START, JoypadKey::Start),
];

/// Process keyboard events
pub struct Keyboard {
    /// Record pressed keys, each bit represent a button status, 1 is pressed, 0 is released
    pub pressed_key: u8,
}

impl Keyboard {
    pub fn create() -> Self {
        Self {
            pressed_key: 0x00,
        }
    }

    /// Return true if [btn] pressed
    pub fn is_button_pressed(&self, btn: GbBtn) -> bool {
        self.pressed_key & btn as u8 != 0x00
    }

    pub fn press_button(&mut self, btn: GbBtn) {
        self.pressed_key |= btn as u8;
    }

    pub fn release_button(&mut self, btn: GbBtn) {
        self.pressed_key &= !(btn as u8);
    }
}