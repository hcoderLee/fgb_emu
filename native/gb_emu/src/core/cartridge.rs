use std::fs::File;
use std::io::prelude::*;
use std::time::SystemTime;
use std::path::{Path, PathBuf};
use crate::core::memory::Memory;
use crate::core::convention::Term;

pub struct RomOnly {
    rom: Vec<u8>,
}

pub trait Stable {
    fn save(&self);
}


impl RomOnly {
    pub fn power_up(rom: Vec<u8>) -> Self {
        RomOnly { rom }
    }
}

impl Memory for RomOnly {
    fn get(&self, a: u16) -> u8 {
        self.rom[a as usize]
    }

    fn set(&mut self, _: u16, _: u8) {
        panic!("cannot set memory in RomOnly");
    }
}

impl Stable for RomOnly {
    fn save(&self) {}
}

enum BankMod {
    Rom,
    Ram,
}

pub struct Mbc1 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    bank_mode: BankMod,
    bank: u8,
    ram_enable: bool,
    save_path: PathBuf,
}

impl Mbc1 {
    pub fn power_up<T: AsRef<Path>>(rom: Vec<u8>, ram: Vec<u8>, sav: T) -> Self {
        Mbc1 {
            rom,
            ram,
            bank_mode: BankMod::Rom,
            bank: 0x01,
            ram_enable: false,
            save_path: PathBuf::from(sav.as_ref()),
        }
    }

    fn rom_bank(&self) -> usize {
        let n = match self.bank_mode {
            BankMod::Ram => self.bank & 0x1f,
            BankMod::Rom => self.bank & 0x7f,
        };
        n as usize
    }

    fn ram_bank(&self) -> usize {
        let n = match self.bank_mode {
            BankMod::Ram => (self.bank & 0x60) >> 5,
            BankMod::Rom => 0x00,
        };
        n as usize
    }
}

impl Memory for Mbc1 {
    fn get(&self, a: u16) -> u8 {
        match a {
            0x0000..=0x3fff => self.rom[a as usize],
            0x4000..=0x7fff => {
                let i = self.rom_bank() * 0x4000 + a as usize - 0x4000;
                self.rom[i]
            }
            0xa000..=0xbfff => {
                if self.ram_enable {
                    let i = self.ram_bank() * 0x2000 + a as usize - 0xa000;
                    self.ram[i]
                } else {
                    0x00
                }
            }
            _ => 0x00,
        }
    }

    fn set(&mut self, a: u16, v: u8) {
        match a {
            0xa000..=0xbfff => {
                if self.ram_enable {
                    let i = self.ram_bank() * 0x2000 + a as usize - 0xa000;
                    self.ram[i] = v;
                }
            }
            0x0000..=0x1fff => {
                self.ram_enable = v & 0x0f == 0x0a;
                if !self.ram_enable {
                    self.save();
                }
            }
            0x2000..=0x3fff => {
                let mut n = v & 0x1f;
                if n == 0x00 {
                    n = 0x01;
                }
                self.bank = self.bank & 0xe0 | n;
            }
            0x4000..=0x5fff => {
                let n = v & 0x03;
                self.bank = self.bank & 0x9f | (n << 5);
            }
            0x6000..=0x7fff => match v {
                0x00 => self.bank_mode = BankMod::Rom,
                0x01 => self.bank_mode = BankMod::Ram,
                _ => panic!("Invalid cartridge type {}", v),
            },
            _ => {}
        }
    }
}

impl Stable for Mbc1 {
    fn save(&self) {
        if self.save_path.to_str().unwrap().is_empty() {
            return;
        }

        File::create(self.save_path.clone())
            .and_then(|mut f| f.write_all(&self.ram))
            .unwrap()
    }
}

pub struct Mbc2 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank: usize,
    ram_enable: bool,
    save_path: PathBuf,
}

impl Mbc2 {
    pub fn power_up(rom: Vec<u8>, ram: Vec<u8>, sav: impl AsRef<Path>) -> Self {
        Self {
            rom,
            ram,
            rom_bank: 1,
            ram_enable: false,
            save_path: PathBuf::from(sav.as_ref()),
        }
    }
}

impl Memory for Mbc2 {
    fn get(&self, a: u16) -> u8 {
        match a {
            0x000..=0x3fff => self.rom[a as usize],
            0x4000..=0x7fff => {
                let i = self.rom_bank * 0x4000 + a as usize - 0x4000;
                self.rom[i]
            }
            0xa000..=0xa1ff => {
                if self.ram_enable {
                    self.ram[(a - 0xa000) as usize]
                } else {
                    0x00
                }
            }
            _ => 0x00,
        }
    }

    fn set(&mut self, a: u16, b: u8) {
        let v = b & 0x0f;
        match a {
            0xa000..=0xa1ff => {
                if self.ram_enable {
                    self.ram[(a - 0xa000) as usize] = v;
                }
            }
            0x000..=0x1fff => {
                if a & 0x0100 == 0 {
                    self.ram_enable = v == 0x0a;
                }
            }
            0x2000..=0x3fff => {
                if a & 0x0100 != 0 {
                    self.rom_bank = v as usize;
                }
            }
            _ => {}
        }
    }
}

impl Stable for Mbc2 {
    fn save(&self) {
        if self.save_path.to_str().unwrap().is_empty() {
            return;
        }
        File::create(self.save_path.clone())
            .and_then(|mut f| f.write_all(&self.ram))
            .unwrap();
    }
}

struct RealTimeClock {
    s: u8,
    m: u8,
    h: u8,
    dl: u8,
    dh: u8,
    zero: u64,
    sav_path: PathBuf,
}

impl RealTimeClock {
    fn power_up(sav_path: impl AsRef<Path>) -> Self {
        let zero = match std::fs::read(sav_path.as_ref()) {
            Ok(v) => {
                let mut b: [u8; 8] = Default::default();
                b.copy_from_slice(&v);
                u64::from_be_bytes(b)
            }
            Err(_) => SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        };
        RealTimeClock {
            s: 0,
            m: 0,
            h: 0,
            dl: 0,
            dh: 0,
            zero,
            sav_path: sav_path.as_ref().to_path_buf(),
        }
    }

    fn tick(&mut self) {
        let d = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() - self.zero;
        self.s = (d % 60) as u8;
        self.m = (d / 60 % 60) as u8;
        self.h = (d / 3600 % 24) as u8;
        let days = d / 3600 / 24;
        self.dl = (days % 256) as u8;
        match days {
            0x0000..=0x00ff => {}
            0x0100..=0x01ff => self.dh |= 0x01,
            _ => self.dh |= 0x81,
        }
    }
}

impl Memory for RealTimeClock {
    fn get(&self, a: u16) -> u8 {
        match a {
            0x08 => self.s,
            0x09 => self.m,
            0x0a => self.h,
            0x0b => self.dl,
            0x0c => self.dh,
            _ => panic!("No entry"),
        }
    }

    fn set(&mut self, a: u16, b: u8) {
        match a {
            0x08 => self.s = b,
            0x09 => self.m = b,
            0x0a => self.h = b,
            0x0b => self.dl = b,
            0x0c => self.dh = b,
            _ => panic!("No entry"),
        }
    }
}

impl Stable for RealTimeClock {
    fn save(&self) {
        if self.sav_path.to_str().unwrap().is_empty() {
            return;
        }
        File::create(self.sav_path.clone())
            .and_then(|mut f| f.write_all(&self.zero.to_be_bytes()))
            .unwrap();
    }
}

struct Mbc3 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank: usize,
    ram_bank: usize,
    ram_enable: bool,
    rtc: RealTimeClock,
    sav_path: PathBuf,
}

impl Mbc3 {
    fn power_up(rom: Vec<u8>, ram: Vec<u8>, sav: impl AsRef<Path>, rtc: impl AsRef<Path>) -> Self {
        Mbc3 {
            rom,
            ram,
            rom_bank: 1,
            ram_bank: 0,
            ram_enable: false,
            rtc: RealTimeClock::power_up(rtc.as_ref().to_path_buf()),
            sav_path: sav.as_ref().to_path_buf(),
        }
    }
}

impl Memory for Mbc3 {
    fn get(&self, a: u16) -> u8 {
        match a {
            0x0000..=0x3fff => self.rom[a as usize],
            0x4000..=0x7fff => {
                // let mut n = self.rom_bank & 0x7f;
                // if n == 0x00 {
                //     n = 0x01;
                // }
                let i = self.rom_bank * 0x4000 + a as usize - 0x4000;
                self.rom[i]
            }
            0xa000..=0xbfff => {
                if self.ram_enable {
                    if self.ram_bank <= 0x03 {
                        let i = self.ram_bank * 0x2000 + a as usize - 0xa000;
                        self.ram[i]
                    } else {
                        self.rtc.get(self.ram_bank as u16)
                    }
                } else {
                    0x00
                }
            }
            _ => 0x00,
        }
    }

    fn set(&mut self, a: u16, b: u8) {
        match a {
            0xa000..=0xbfff => {
                if !self.ram_enable {
                    return;
                }
                if self.ram_bank <= 0x03 {
                    let i = self.ram_bank * 0x2000 + a as usize - 0xa000;
                    self.ram[i] = b;
                } else {
                    self.rtc.set(self.ram_bank as u16, b);
                }
            }
            0x0000..=0x1fff => self.ram_enable = b & 0x0f == 0x0a,
            0x2000..=0x3fff => {
                let v = if b == 0x00 { 0x01 } else { b };
                self.rom_bank = (v & 0x7f) as usize;
            }
            0x4000..=0x5fff => self.ram_bank = (b & 0x0f) as usize,
            0x6000..=0x7fff => {
                if b & 0x01 != 0 {
                    self.rtc.tick();
                }
            }
            _ => {}
        }
    }
}

impl Stable for Mbc3 {
    fn save(&self) {
        self.rtc.save();
        if self.sav_path.to_str().unwrap().is_empty() {
            return;
        }
        File::create(self.sav_path.clone())
            .and_then(|mut f| f.write_all(&self.ram))
            .unwrap();
    }
}

struct Mbc5 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank: usize,
    ram_bank: usize,
    ram_enable: bool,
    sav_path: PathBuf,
}

impl Mbc5 {
    fn power_up(rom: Vec<u8>, ram: Vec<u8>, sav: impl AsRef<Path>) -> Self {
        Mbc5 {
            rom,
            ram,
            rom_bank: 1,
            ram_bank: 0,
            ram_enable: false,
            sav_path: PathBuf::from(sav.as_ref()),
        }
    }
}

impl Memory for Mbc5 {
    fn get(&self, a: u16) -> u8 {
        match a {
            0x0000..=0x3fff => self.rom[a as usize],
            0x4000..=0x7fff => {
                let i = self.rom_bank * 0x4000 + a as usize - 0x4000;
                self.rom[i]
            }
            0xa000..=0xbfff => {
                if self.ram_enable {
                    let i = self.ram_bank * 0x2000 + a as usize - 0xa000;
                    self.ram[i]
                } else {
                    0x00
                }
            }
            _ => 0x00,
        }
    }

    fn set(&mut self, a: u16, b: u8) {
        match a {
            0xa000..=0xbfff => {
                if self.ram_enable {
                    let i = self.ram_bank * 0x2000 + a as usize - 0xa000;
                    self.ram[i] = b;
                }
            }
            0x0000..=0x1fff => self.ram_enable = (b & 0x0f) == 0x0a,
            0x2000..=0x2fff => self.rom_bank = (self.rom_bank & 0x0100) | b as usize,
            0x3000..=0x3fff => self.rom_bank = (self.rom_bank & 0x00ff) | ((b as usize & 0x01) << 8),
            0x4000..=0x5fff => self.ram_bank = b as usize & 0x0f,
            _ => {}
        }
    }
}

impl Stable for Mbc5 {
    fn save(&self) {
        if self.sav_path.to_str().unwrap().is_empty() {
            return;
        }
        File::create(self.sav_path.clone())
            .and_then(|mut f| f.write_all(&self.ram))
            .unwrap();
    }
}

pub trait Cartridge: Memory + Stable + Send {
    // 获取卡带标题
    fn title(&self) -> String {
        let mut buf = String::new();
        let ic = 0x0134;
        let oc = if self.get(0x0143) == 0x80 { 0x013e } else { 0x0143 };
        for i in ic..oc {
            match self.get(i) {
                0 => break,
                v => buf.push(v as char),
            }
        }
        buf
    }

    fn term(&self) -> Term {
        if self.get(0x0143) & 0x80 != 0 {
            Term::GBC
        } else {
            Term::GB
        }
    }
}

// 初始化卡带
pub fn power_up(path: impl AsRef<Path>) -> Box<dyn Cartridge> {
    let mut f = File::open(path.as_ref()).unwrap();
    let mut rom = vec![];
    f.read_to_end(&mut rom).unwrap();
    if rom.len() < 0x150 {
        panic!("Missing required information area which located at 0100-014F")
    }
    let rom_max = rom_size(rom[0x0148]);
    if rom.len() > rom_max {
        panic!("Rom size more than: {}", rom_max)
    }
    let cart: Box<dyn Cartridge> = match rom[0x0147] {
        0x00 => Box::new(RomOnly::power_up(rom)),
        0x01 => Box::new(Mbc1::power_up(rom, vec![], "")),
        0x02 => {
            let ram_max = ram_size(rom[0x149]);
            Box::new(Mbc1::power_up(rom, vec![0; ram_max], ""))
        }
        0x03 => {
            let ram_max = ram_size(rom[0x149]);
            let sav_path = path.as_ref().with_extension("sav");
            let ram = ram_read(sav_path.clone(), ram_max);
            Box::new(Mbc1::power_up(rom, ram, sav_path))
        }
        0x05 => {
            let ram_max = 512;
            Box::new(Mbc2::power_up(rom, vec![0; ram_max], ""))
        }
        0x06 => {
            let ram_max = 512;
            let sav_path = path.as_ref().with_extension("sav");
            let ram = ram_read(sav_path.clone(), ram_max);
            Box::new(Mbc2::power_up(rom, ram, sav_path))
        }
        0x0f => {
            let sav_path = path.as_ref().with_extension("sav");
            let rtc_path = path.as_ref().with_extension("rtc");
            Box::new(Mbc3::power_up(rom, vec![], sav_path, rtc_path))
        }
        0x10 => {
            let sav_path = path.as_ref().with_extension("sav");
            let rtc_path = path.as_ref().with_extension("rtc");
            let ram_max = ram_size(rom[0x149]);
            let ram = ram_read(sav_path.clone(), ram_max);
            Box::new(Mbc3::power_up(rom, ram, sav_path, rtc_path))
        }
        0x11 => Box::new(Mbc3::power_up(rom, vec![], "", "")),
        0x12 => {
            let ram_max = ram_size(rom[0x149]);
            Box::new(Mbc3::power_up(rom, vec![0; ram_max], "", ""))
        }
        0x13 => {
            let ram_max = ram_size(rom[0x149]);
            let sav_path = path.as_ref().with_extension("sav");
            let ram = ram_read(sav_path.clone(), ram_max);
            Box::new(Mbc3::power_up(rom, ram, sav_path, ""))
        }
        0x19 => Box::new(Mbc5::power_up(rom, vec![], "")),
        0x1a => {
            let ram_max = ram_size(rom[0x149]);
            Box::new(Mbc5::power_up(rom, vec![0; ram_max], ""))
        }
        0x1b => {
            let ram_max = ram_size(rom[0x149]);
            let sav_path = path.as_ref().with_extension("sav");
            let ram = ram_read(sav_path.clone(), ram_max);
            Box::new(Mbc5::power_up(rom, ram, sav_path))
        }
        n => panic!("Unsupported cartridge type: {:#04x}", n),
    };
    println!("Cartridge title: {}", cart.title());
    println!("Cartridge type: {}", mbc_info(cart.as_ref()));
    ensure_header_checksum(cart.as_ref());
    ensure_logo(cart.as_ref());
    cart
}

fn mbc_info(cart: &dyn Cartridge) -> String {
    let ty = cart.get(0x147);
    String::from(match ty {
        0x00 => "ROM ONLY",
        0x01 => "MBC1",
        0x02 => "MBC1+RAM",
        0x03 => "MBC1+RAM+BATTERY",
        0x05 => "MBC2",
        0x06 => "MBC2+BATTERY",
        0x08 => "ROM+RAM",
        0x09 => "ROM+RAM+BATTERY",
        0x0b => "MMM01",
        0x0c => "MMM01+RAM",
        0x0d => "MMM01+RAM+BATTERY",
        0x0f => "MBC3+TIMER+BATTERY",
        0x10 => "MBC3+TIMER+RAM+BATTERY",
        0x11 => "MBC3",
        0x12 => "MBC3+RAM",
        0x13 => "MBC3+RAM+BATTERY",
        0x15 => "MBC4",
        0x16 => "MBC4+RAM",
        0x17 => "MBC4+RAM+BATTERY",
        0x19 => "MBC5",
        0x1a => "MBC5+RAM",
        0x1b => "MBC5+RAM+BATTERY",
        0x1c => "MBC5+RUMBLE",
        0x1d => "MBC5+RUMBLE+RAM",
        0x1e => "MBC5+RUMBLE+RAM+BATTERY",
        0xfc => "POCKET CAMERA",
        0xfd => "BANDAI TAMA5",
        0xfe => "HuC3",
        0x1f => "HuC1+RAM+BATTERY",
        n => panic!("Unsupported cartridge type: 0x{:02x}", n),
    })
}


const NINTENDO_LOGO: [u8; 48] = [
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83,
    0x00, 0x0C, 0x00, 0x0D, 0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E,
    0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99, 0xBB, 0xBB, 0x67, 0x63,
    0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
];

// 验证任天堂logo
fn ensure_logo(cart: &dyn Cartridge) {
    for i in 0..48 {
        if cart.get(0x0104 + i) != NINTENDO_LOGO[i as usize] {
            panic!("Nintendo logo is incorrect!")
        }
    }
}

// 验证标题校验和
fn ensure_header_checksum(cart: &dyn Cartridge) {
    let mut v: u8 = 0;
    for i in 0x0134..0x014d {
        v = v.wrapping_sub(cart.get(i)).wrapping_sub(1);
    }

    if cart.get(0x014d) != v {
        panic!("Cartridge's header checksum is incorrect!")
    }
}

// 获取卡带中rom的容量
fn rom_size(b: u8) -> usize {
    let bank = 16384;
    match b {
        0x00 => bank * 2,
        0x01 => bank * 4,
        0x02 => bank * 8,
        0x03 => bank * 16,
        0x04 => bank * 32,
        0x05 => bank * 64,
        0x06 => bank * 128,
        0x07 => bank * 256,
        0x08 => bank * 512,
        0x52 => bank * 72,
        0x53 => bank * 80,
        0x54 => bank * 96,
        n => panic!("Unsupported rom size: 0x{:02x}", n),
    }
}

// 获取卡带中ram的容量
fn ram_size(b: u8) -> usize {
    match b {
        0x00 => 0,
        0x01 => 1024 * 2,
        0x02 => 1024 * 8,
        0x03 => 1024 * 32,
        0x04 => 1024 * 128,
        0x05 => 1024 * 64,
        n => panic!("Unsupported ram size: 0x{:02x}", n),
    }
}

fn ram_read(sav: impl AsRef<Path>, size: usize) -> Vec<u8> {
    match File::open(sav) {
        Ok(mut f) => {
            let mut ram = vec![];
            f.read_to_end(&mut ram).unwrap();
            ram
        }
        Err(_) => vec![0; size],
    }
}

impl Cartridge for RomOnly {}

impl Cartridge for Mbc1 {}

impl Cartridge for Mbc2 {}

impl Cartridge for Mbc3 {}

impl Cartridge for Mbc5 {}