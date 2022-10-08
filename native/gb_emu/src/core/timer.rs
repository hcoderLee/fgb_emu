use std::rc::Rc;
use std::cell::RefCell;
use crate::core::intf::{Intf, INTFlag};
use crate::core::clock::Clock;
use crate::core::convention::CPU_FREQ;
use crate::core::memory::Memory;

/// 定时器，直接与内存管理模块相连，定期中断CPU执行，使CPU已固定频率执行某些工作
pub struct Timer {
    intf: Rc<RefCell<Intf>>,
    /// DIV (Divider Register)寄存器以16MHZ的频率递增，任何写入该寄存器值都会将其重置为0x00
    div: u8,
    /// 控制DIV自增的时钟
    div_clock: Clock,
    /// TIMA (Time counter)寄存器, 以TAC寄存器指定的频率递增，当值溢出时，将其重置为TMA寄存器的值，并请求CPU中断
    tima: u8,
    /// TMA (Timer Modulo)寄存器
    tma: u8,
    /// TAC (Timer Control)寄存器
    /// Bit 2: 是否启用，1表示启用，0表示禁用
    /// Bit 1~0: 设定的定时器频率
    /// 0: CPU Clock / 1024 (DMG, CGB: 4096 Hz, SGB: ~4194 Hz)
    /// 1: CPU Clock / 16 (DMG, CGB: 262144 Hz, SGB: ~268400 Hz)
    /// 2: CPU Clock / 64 (DMG, CGB: 65536 Hz, SGB: ~67110 Hz)
    /// 3: CPU Clock / 256 (DMG, CGB: 16384 Hz, SGB: ~16780 Hz)
    tac: u8,
    /// TAC寄存器控制的时钟
    timer_clock: Clock,
}

impl Timer {
    pub fn power_up(intf: Rc<RefCell<Intf>>) -> Self {
        Self {
            intf,
            div: 0x00,
            div_clock: Clock::power_up(CPU_FREQ / (16 * 1024)),
            tima: 0x00,
            tma: 0x00,
            tac: 0x00,
            timer_clock: Clock::power_up(1024),
        }
    }

    pub fn next(&mut self, cycles: u32) {
        // 增加DIV寄存器的值
        self.div = self.div.wrapping_add(self.div_clock.next(cycles) as u8);
        if self.tac & 0x04 == 0 {
            // 未启用定时器
            return;
        }
        // 增加TIMA寄存器的值，找出溢出的时机
        let n = self.timer_clock.next(cycles);
        for _ in 0..n {
            self.tima = self.tima.wrapping_add(1);
            if self.tima == 0x00 {
                // 将TIMA寄存器的值重置为TMA中的值
                self.tima = self.tma;
                // TIMA寄存器溢出，请求CPU中断
                self.intf.borrow_mut().hi(INTFlag::Timer);
            }
        }
    }
}

impl Memory for Timer {
    fn get(&self, a: u16) -> u8 {
        match a {
            0xff04 => self.div,
            0xff05 => self.tima,
            0xff06 => self.tma,
            0xff07 => self.tac,
            _ => unreachable!(),
        }
    }

    fn set(&mut self, a: u16, v: u8) {
        match a {
            0xff04 => {
                // 任何写入DIV寄存器的行为，会将其重置为0x00
                self.div = 0x00;
                // 重置控制DIV寄存器自增的时钟
                self.div_clock.n = 0;
            }
            0xff05 => self.tima = v,
            0xff06 => self.tma = v,
            0xff07 => {
                if self.tac & 0x03 != v & 0x03 {
                    // 修改定时器时钟频率
                    self.timer_clock.period = match v & 0x03 {
                        0 => 1024,
                        1 => 16,
                        2 => 64,
                        3 => 256,
                        _ => unreachable!(),
                    };
                    // 重置定时器时钟
                    self.timer_clock.n = 0;
                    // 重置TIMA寄存器的值
                    self.tima = self.tma;
                }
                self.tac = v;
            }
            _ => unreachable!(),
        }
    }
}