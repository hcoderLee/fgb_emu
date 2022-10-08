use crate::core::memory::Memory;

/// 串行数据传输
/// 这里并没有实现实际的数据传输逻辑
pub struct Serial {
    /// 在传输前，保存下一个要发送的字节
    /// 在传输中，它混合了输出和输入的字节，每个时钟周期中，数据从左侧移出，通过线缆发送出去，新数据从另一侧写入
    data: u8,
    /// 控制串口数据传输
    /// Bit 7: 开始传输标志，0表示没有正在进行或已请求的传输，1表示正在传输中或已请求传输
    /// Bit 1: 时钟速度，0表示Normal，1表示Fast（仅CGB模式）
    /// Bit 0: 移位时钟，0表示外部时钟，1表示内部时钟
    control: u8,
}

impl Serial {
    pub fn power_up() -> Self {
        Self {
            data: 0x00,
            control: 0x00,
        }
    }
}

impl Memory for Serial {
    fn get(&self, a: u16) -> u8 {
        match a {
            0xff01 => self.data,
            0xff02 => self.control,
            _ => unreachable!(),
        }
    }

    fn set(&mut self, a: u16, v: u8) {
        match a {
            0xff01 => self.data = v,
            0xff02 => self.control = v,
            _ => unreachable!(),
        }
    }
}