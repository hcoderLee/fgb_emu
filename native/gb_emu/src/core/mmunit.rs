use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use crate::core::apu::APU;
use crate::core::cartridge;
use crate::core::cartridge::Cartridge;
use crate::core::convention::Term;
use crate::core::dma::{DMAMode, DMA};
use crate::core::gpu::GPU;
use crate::core::hram::HRAM;
use crate::core::intf::Intf;
use crate::core::joypad::Joypad;
use crate::core::memory::Memory;
use crate::core::serial::Serial;
use crate::core::speed::Speed;
use crate::core::timer::Timer;
use crate::core::wram::WRAM;

// 内存管理单元，用于将所有外设的存储空间拼接成一段连续的内存空间，对外提供统一的内存访问接口
pub struct MMUnit {
    // 卡带
    pub cartridge: Box<dyn Cartridge>,
    // 音频处理器
    pub apu: Option<APU>,
    // 视频处理器
    pub gpu: GPU,
    // 手柄控制器
    pub joypad: Joypad,
    // 串行通信，与其他GB交换数据
    pub serial: Serial,
    // 与speed一起处理运行速度（单/双倍速）
    pub shift: bool,
    // 与shift一起处理运行速度（单/双倍速）
    pub speed: Speed,
    // GB型号
    pub term: Term,
    // 定时器
    pub timer: Timer,
    // 是否允许特定类型的中断
    inte: u8,
    // 是否发生特定类型的中断
    intf: Rc<RefCell<Intf>>,
    // 用于从ROM等其他区域copy数据到OAM内存区域或VRAM内存区域
    dma: DMA,
    // 视频GPU的一部分
    wram: WRAM,
    hram: HRAM,
}

impl MMUnit {
    pub fn power_up<T: AsRef<Path>>(path: T, save_path: T) -> Self {
        let cartridge = cartridge::power_up(path, save_path);
        let term = cartridge.term();
        let intf = Rc::new(RefCell::new(Intf::power_up()));
        let mut mmunit = Self {
            cartridge,
            apu: None,
            gpu: GPU::power_up(term, intf.clone()),
            joypad: Joypad::power_up(intf.clone()),
            serial: Serial::power_up(),
            shift: false,
            speed: Speed::power_up(),
            term,
            timer: Timer::power_up(intf.clone()),
            inte: 0x00,
            intf: intf.clone(),
            dma: DMA::power_up(),
            wram: WRAM::power_up(),
            hram: HRAM::power_up(),
        };
        mmunit.init();
        return mmunit;
    }

    /// 初始化某些内存的数据
    fn init(&mut self) {
        self.set(0xff05, 0x00);
        self.set(0xff06, 0x00);
        self.set(0xff07, 0x00);
        self.set(0xff10, 0x80);
        self.set(0xff11, 0xbf);
        self.set(0xff12, 0xf3);
        self.set(0xff14, 0xbf);
        self.set(0xff16, 0x3f);
        self.set(0xff16, 0x3f);
        self.set(0xff17, 0x00);
        self.set(0xff19, 0xbf);
        self.set(0xff1a, 0x7f);
        self.set(0xff1b, 0xff);
        self.set(0xff1c, 0x9f);
        self.set(0xff1e, 0xff);
        self.set(0xff20, 0xff);
        self.set(0xff21, 0x00);
        self.set(0xff22, 0x00);
        self.set(0xff23, 0xbf);
        self.set(0xff24, 0x77);
        self.set(0xff25, 0xf3);
        self.set(0xff26, 0xf1);
        self.set(0xff40, 0x91);
        self.set(0xff42, 0x00);
        self.set(0xff43, 0x00);
        self.set(0xff45, 0x00);
        self.set(0xff47, 0xfc);
        self.set(0xff48, 0xff);
        self.set(0xff49, 0xff);
        self.set(0xff4a, 0x00);
        self.set(0xff4b, 0x00);
    }
}

impl MMUnit {
    pub fn next(&mut self, cycles: u32) -> u32 {
        let cpu_speed = self.speed.mode as u32;
        let dma_cost = self.run_dma();
        let gpu_cycles = cycles / cpu_speed + dma_cost;
        let cpu_cycles = cycles + dma_cost * cpu_speed;
        self.timer.next(cpu_cycles);
        self.gpu.next(gpu_cycles);
        // if let Some(apu) = &mut self.apu
        // {
        //     apu.next(gpu_cycles);
        // }
        return gpu_cycles;
    }

    /// 执行dma数据拷贝，返回消耗的CPU时钟周期
    fn run_dma(&mut self) -> u32 {
        if !self.dma.active {
            return 0;
        }
        match self.dma.mode {
            DMAMode::GDMA => {
                let len = u32::from(self.dma.remain) + 1;
                // GDMA模式一次性拷贝完所有的数据
                for _ in 0..len {
                    self.run_dma_hram_copy();
                }
                // 数据拷贝完成
                self.dma.active = false;
                len * 8
            }
            DMAMode::HDMA => {
                if !self.gpu.h_blank {
                    // 非HBlank模式下HDMA不可用
                    return 0;
                }
                // HDMA模式在HBlank期间只拷贝一次数据
                self.run_dma_hram_copy();
                // 每次拷贝数据花费8个CPU时钟周期
                8
            }
        }
    }

    /// 执行一次DMA数据拷贝，最终数据将被拷贝到HRAM区域
    fn run_dma_hram_copy(&mut self) {
        // DMA一次拷贝0x10个字节的数据
        for i in 0..0x10 {
            let data = self.get(self.dma.src + i);
            self.gpu.set(self.dma.dst + i, data);
        }
        // 更新DMA的起始和目标地址
        self.dma.src += 0x10;
        self.dma.dst += 0x10;
        if self.dma.remain == 0 {
            self.dma.remain = 0x7f;
        } else {
            self.dma.remain -= 1;
        }
    }
}

impl Memory for MMUnit {
    fn get(&self, a: u16) -> u8 {
        match a {
            // 卡带
            0x0000..=0x7fff => self.cartridge.get(a),
            // GPU
            0x8000..=0x9fff => self.gpu.get(a),
            // 卡带
            0xa000..=0xbfff => self.cartridge.get(a),
            // WRAM
            0xc000..=0xfdff => self.wram.get(a),
            // GPU
            0xfe00..=0xfe9f => self.gpu.get(a),
            // Unused area
            0xfea0..=0xfeff => 0x00,
            // 手柄
            0xff00 => self.joypad.get(a),
            // 串口通信
            0xff01..=0xff02 => self.serial.get(a),
            // 定时器
            0xff04..=0xff07 => self.timer.get(a),
            // 中断
            0xff0f => self.intf.borrow().data,
            // 音频
            0xff10..=0xff3f => match &self.apu {
                Some(apu) => apu.get(a),
                None => 0x00,
            },
            // Speed
            0xff4d => self.speed.get(a),
            // GPU
            0xff40..=0xff45 | 0xff47..=0xff4b | 0xff4f => self.gpu.get(a),
            // DMA
            0xff51..=0xff55 => self.dma.get(a),
            // GPU
            0xff68..=0xff6b => self.gpu.get(a),
            // WRAM bank
            0xff70 => self.wram.get(a),
            // HRAM
            0xff80..=0xfffe => self.hram.get(a),
            // 是否允许中断
            0xffff => self.inte,
            _ => 0x00,
        }
    }

    fn set(&mut self, a: u16, v: u8) {
        match a {
            // 卡带
            0x0000..=0x7fff => self.cartridge.set(a, v),
            // GPU
            0x8000..=0x9fff => self.gpu.set(a, v),
            // 卡带
            0xa000..=0xbfff => self.cartridge.set(a, v),
            // WRAM
            0xc000..=0xfdff => self.wram.set(a, v),
            // GPU
            0xfe00..=0xfe9f => self.gpu.set(a, v),
            // Unused
            0xfea0..=0xfeff => {}
            // 手柄
            0xff00 => self.joypad.set(a, v),
            // 串口通信
            0xff01..=0xff02 => self.serial.set(a, v),
            // 定时器
            0xff04..=0xff07 => self.timer.set(a, v),
            // 中断
            0xff0f => self.intf.borrow_mut().data = v,
            // 音频
            0xff10..=0xff3f => {
                if let Some(apu) = &mut self.apu {
                    apu.set(a, v);
                }
            }
            0xff46 => {
                // 写入此寄存器将触发DMA数据传输
                //  Source:      XX00-XX9F   ;XX in range from 00-F1h
                //  Destination: FE00-FE9F
                assert!(v <= 0xf1);
                let base = u16::from(v) << 8;
                for i in 0..0xa0 {
                    let b = self.get(base + i);
                    self.set(0xfe00 + i, b);
                }
            }
            // Speed
            0xff4d => self.speed.set(a, v),
            // GPU
            0xff40..=0xff45 | 0xff47..=0xff4b | 0xff4f => self.gpu.set(a, v),
            // DMA
            0xff51..=0xff55 => self.dma.set(a, v),
            // GPU
            0xff68..=0xff6b => self.gpu.set(a, v),
            // 设置WRAM bank
            0xff70 => self.wram.set(a, v),
            // HRAM
            0xff80..=0xfffe => self.hram.set(a, v),
            // 设置是否允许中断
            0xffff => self.inte = v,
            _ => {}
        }
    }
}
