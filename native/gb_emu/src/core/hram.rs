use crate::core::memory::Memory;

pub struct HRAM {
    hram: [u8; 0x7f],
}

impl HRAM {
    pub fn power_up() -> Self {
        Self {
            hram: [0x00; 0x7f],
        }
    }
}

impl Memory for HRAM {
    fn get(&self, a: u16) -> u8 {
        match a {
            0xff80..=0xfffe => self.hram[a as usize - 0xff80],
            _ => unreachable!(),
        }
    }

    fn set(&mut self, a: u16, v: u8) {
        match a {
            0xff80..=0xfffe => self.hram[a as usize - 0xff80] = v,
            _ => unreachable!(),
        }
    }
}