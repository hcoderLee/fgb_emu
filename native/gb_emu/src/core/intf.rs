/// GB支持的中断类型
pub enum INTFlag {
    VBlank = 0,
    LCDStat = 1,
    Timer = 2,
    Serial = 3,
    Joypad = 4,
}

/// IF寄存器，用于保存当前已产生的中断请求
/// 映射在内存地址：0xff0f
pub struct Intf {
    pub data: u8,
}

impl Intf {
    pub fn power_up() -> Self {
        Intf { data: 0x00 }
    }

    /// 收到中断时置位
    pub fn hi(&mut self, flag: INTFlag) {
        self.data |= 1 << flag as u8;
    }
}