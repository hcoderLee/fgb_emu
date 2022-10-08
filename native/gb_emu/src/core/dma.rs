use std::fmt::{Display, Formatter};
use crate::core::dma::DMAMode::{GDMA, HDMA};
use crate::core::memory::Memory;

#[derive(Eq, PartialEq)]
pub enum DMAMode {
    /// General Purpose DMA, 在此模式下数据将一次性传输完毕，程序将暂停运行直到数据传输完成，就算是LCD控制器在访问
    /// VRAM时，GDMA也会盲目的复制数据，所以GDMA只能在LCD不可用时使用，或者在VBlank，HBlank期间使用
    /// 在GDMA模式下，传输数据结束后，程序继续执行，此时从FF55内存地址读取的数据是: 0xFF
    GDMA,
    /// HBlank DMA, 此模式在每次HBlank期间传输0x10个字节（即ly在0～143之间才传输数据, 当ly在144～153范围内时，
    /// 不传输数据，在ly=0时重新开始传输数据）。程序在每次数据传输的时都会被暂停，并在数据传输完成的"空隙"内恢复执行
    /// 在数据传输完成前不能改变VRAM bank的值
    /// 假设剩下r个字节的数据要传输，内存地址0xFF55中的值为x，则: x = r / 0x10 - 1, 如果x=0xff 则表示数据传输完
    /// 毕。如果想要结束一个active状态的HBlank传输，可以将内存地址0xFF55的第7为置0，在这种情况下读取0xFF55时，第7
    /// 位则会被读为1
    HDMA,
}

impl Display for DMAMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GDMA => write!(f, "GDMA"),
            HDMA => write!(f, "HDMA"),
        }
    }
}

/// Direct memory access, 用于从ROM等其他区域copy数据到OAM内存区域或VRAM内存区域
pub struct DMA {
    /// 从哪个内存地址开始copy数据，通常在ROM，SRAM，WRAM等区域，地址区间为：0x0000-0x7FF0 或 0xA000-0xDFF0
    pub src: u16,
    /// 数据将要copy到哪个地址，取值范围是VRAM的地址区间: 0x8000~0x9FF0内
    /// 寄存器的低4位和高3位将被忽略，只有12～4位有效（0~0x1FF0），最高温默认位1，这样最终的取值范围刚好落在0x8000~0x9FF0
    pub dst: u16,
    /// HDMA是否可以正常传输数据，如果被终止则为false
    pub active: bool,
    /// General DMA 或 H-blank DMA
    pub mode: DMAMode,
    /// 剩余要传输的字节数（以0x10个字节为单位），只有低7位有效
    pub remain: u8,
}

impl DMA {
    pub fn power_up() -> Self {
        Self {
            src: 0x0000,
            dst: 0x8000,
            active: false,
            mode: GDMA,
            remain: 0x00,
        }
    }
}

impl Display for DMA {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "src: {:#04x}, dest: {:#04x}, active: {}, mode: {}, remain: {}", self.src, self.dst, self.active, self.mode, self.remain)
    }
}

impl Memory for DMA {
    fn get(&self, a: u16) -> u8 {
        match a {
            // HDMA1寄存器，保存source address的高8位
            0xff51 => (self.src >> 8) as u8,
            // HDMA2寄存器，保存source address的低8位
            0xff52 => self.src as u8,
            // HDMA3寄存器，保存destination address的高8位
            0xff53 => (self.dst >> 8) as u8,
            // HDMA4寄存器，保存destination address的低8位
            0xff54 => self.dst as u8,
            // HDMA5寄存器，读取时返回剩余要传输的数据块数量（数据块长度为0x10）
            // HDMA被终止（active为false），第7位要被视为1
            0xff55 => if self.active { self.remain } else { self.remain | 0x80 },
            _ => panic!("Invalid to read DMA address: {}", a)
        }
    }

    fn set(&mut self, a: u16, v: u8) {
        match a {
            // HDMA1寄存器，设置source address的高8位
            0xff51 => self.src = (u16::from(v) << 8) | (self.src & 0x00ff),
            // HDMA2寄存器，设置source address的低8位，低4位将被忽略
            0xff52 => self.src = u16::from(v & 0xf0) | (self.src & 0xff00),
            // HDMA4寄存器，设置destination address的高8位，高3位被忽略且最高位默认位1
            0xff53 => self.dst = (u16::from(v & 0x1f) << 8) | (self.dst & 0x00ff) | 0x8000,
            // HDMA4寄存器，设置destination address的低8位，低4位将被忽略
            0xff54 => self.dst = u16::from(v & 0xf0) | (self.dst & 0xff00),
            // HDMA5寄存器，用于初始化一次DMA数据传输
            0xff55 => {
                if self.active && self.mode == HDMA {
                    if v & 0x80 == 0 {
                        // HDMA传输数据被中断
                        self.active = false;
                    }
                    return;
                }
                self.active = true;
                // 低7位表示要传输的长度
                self.remain = v & 0x7f;
                self.mode = if v & 0x80 == 0 {
                    // 第7位为0，表示本次数据传输使用GDMA模式
                    GDMA
                } else {
                    // 第7位为1，表示本次数据传输使用HDMA模式
                    HDMA
                };
            }
            _ => panic!("Invalid to set DMA address: {}", a),
        }
    }
}