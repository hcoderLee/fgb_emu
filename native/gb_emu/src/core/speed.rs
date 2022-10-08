use crate::core::memory::Memory;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum SpeedMode {
    Normal = 0x01,
    Double = 0x02,
}

pub struct Speed {
    pub mode: SpeedMode,
    pub prepare_switch: bool,
}

impl Speed {
    pub fn power_up() -> Self {
        Self {
            mode: SpeedMode::Normal,
            prepare_switch: false,
        }
    }

    pub fn switch_speed(&mut self) {
        if !self.prepare_switch {
            return;
        }

        if self.mode == SpeedMode::Double {
            self.mode = SpeedMode::Normal;
        } else {
            self.mode = SpeedMode::Double;
        }
        self.prepare_switch = false;
    }
}

impl Memory for Speed {
    fn get(&self, a: u16) -> u8 {
        match a {
            0xff4d => {
                let a = if self.mode == SpeedMode::Double { 0x80 } else { 0x00 };
                let b = if self.prepare_switch { 0x01 } else { 0x00 };
                a | b
            }
            _ => unreachable!(),
        }
    }

    fn set(&mut self, a: u16, v: u8) {
        match a {
            0xff4d => self.prepare_switch = v & 0x01 != 0,
            _ => unreachable!(),
        }
    }
}
