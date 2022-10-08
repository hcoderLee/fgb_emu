use std::fmt;
use std::fmt::Formatter;
use crate::core::convention::Term;

#[derive(Clone, Default)]
// f 是flag寄存器, 且与a, b, c, d, e, h, l都是8位寄存器
// af, bc, de, hl可两两组合为16位寄存器使用
// sp: stack pointer，指向内存中栈区的顶部
// pc: program counter, 指向下一条要执行指令的内存地址
pub struct Register {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Register {
    pub fn get_af(&self) -> u16 {
        (u16::from(self.a) << 8) | u16::from(self.f)
    }

    pub fn get_bc(&self) -> u16 {
        (u16::from(self.b) << 8) | u16::from(self.c)
    }

    pub fn get_de(&self) -> u16 {
        (u16::from(self.d) << 8) | u16::from(self.e)
    }

    pub fn get_hl(&self) -> u16 {
        (u16::from(self.h) << 8) | u16::from(self.l)
    }

    pub fn set_af(&mut self, v: u16) {
        self.a = (v >> 8) as u8;
        self.f = (v & 0x00f0) as u8;
    }

    pub fn set_bc(&mut self, v: u16) {
        self.b = (v >> 8) as u8;
        self.c = (v & 0x00ff) as u8;
    }

    pub fn set_de(&mut self, v: u16) {
        self.d = (v >> 8) as u8;
        self.e = (v & 0x00ff) as u8;
    }

    pub fn set_hl(&mut self, v: u16) {
        self.h = (v >> 8) as u8;
        self.l = (v & 0x00ff) as u8;
    }
}

pub enum Flag {
    Z = 0b1000_0000,
    N = 0b0100_0000,
    H = 0b0010_0000,
    C = 0b0001_0000,
}

impl Flag {
    pub fn og(self) -> u8 {
        self as u8
    }

    pub fn bw(self) -> u8 {
        !self.og()
    }
}

impl Register {
    pub fn get_flag(&self, f: Flag) -> bool {
        self.f & f.og() != 0
    }

    pub fn set_flag(&mut self, f: Flag, v: bool) {
        if v {
            self.f |= f.og();
        } else {
            self.f &= f.bw();
        }
    }
}

impl Register {
    pub fn power_up(term: Term) -> Self {
        let mut r = Self::default();
        match term {
            Term::GB => r.a = 0x01,
            Term::GBP => r.a = 0xff,
            Term::GBC => r.a = 0x11,
            Term::SGB => r.a = 0x01,
        }
        r.f = 0xb0;
        r.b = 0x00;
        r.c = 0x13;
        r.d = 0x00;
        r.e = 0xd8;
        r.h = 0x01;
        r.l = 0x4d;
        // GameBoy的栈指针在开机时默认指向0xfffe
        r.sp = 0xfffe;
        // GameBoy的program counter在开机时就默认指向0x0100，也就是卡带中rom的地址，保存着一开机就执行的程序
        r.pc = 0x0100;
        r
    }
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "a={} b={} c={} d={} e={} f={} h={} l={} sp={} pc={}", self.a, self.b, self.c, self.d, self.e, self.f, self.h, self.l, self.sp, self.pc)
    }
}
