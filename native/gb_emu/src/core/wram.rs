use crate::core::memory::Memory;

pub struct WRAM {
    wram_bank: usize,
    wram: [u8; 0x8000],
}

impl WRAM {
    pub fn power_up() -> Self {
        Self {
            wram: [0x00; 0x8000],
            wram_bank: 0x01,
        }
    }

    fn set_wram_bank(&mut self, v: u8) {
        self.wram_bank = match v & 0x07 {
            0 => 1,
            n => n as usize,
        }
    }
}

impl Memory for WRAM {
    fn get(&self, a: u16) -> u8 {
        match a {
            0xc000..=0xcfff => self.wram[a as usize - 0xc000],
            0xd000..=0xdfff => self.wram[a as usize - 0xd000 + 0x1000 * self.wram_bank],
            0xe000..=0xefff => self.wram[a as usize - 0xe000],
            0xf000..=0xfdff => self.wram[a as usize - 0xf000 + 0x1000 * self.wram_bank],
            0xff70 => self.wram_bank as u8,
            _ => unreachable!(),
        }
    }

    fn set(&mut self, a: u16, v: u8) {
        match a {
            0xc000..=0xcfff => self.wram[a as usize - 0xc000] = v,
            0xd000..=0xdfff => self.wram[a as usize - 0xd000 + 0x1000 * self.wram_bank] = v,
            0xe000..=0xefff => self.wram[a as usize - 0xe000] = v,
            0xf000..=0xfdff => self.wram[a as usize - 0xf000 + 0x1000 * self.wram_bank] = v,
            0xff70 => self.set_wram_bank(v),
            _ => unreachable!(),
        }
    }
}