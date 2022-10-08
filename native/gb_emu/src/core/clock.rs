/// 期频率与原始时钟周期频率保持一个固定的比值
pub struct Clock {
    /// 多少个原始时钟周期输出一个新的时钟周期
    pub period: u32,
    /// 累计的原始时钟周期
    pub n: u32,
}

/// 根据原始时钟信号触发产生新的时钟信号，新的时钟
impl Clock {
    pub fn power_up(period: u32) -> Self {
        Self { period, n: 0x00 }
    }

    /// cycles为刚刚经历的原始时钟周期，返回相应的新时钟周期
    pub fn next(&mut self, cycles: u32) -> u32 {
        // 累计原始时钟周期
        self.n += cycles;
        // 计算出对应的新时钟周期
        let rs = self.n / self.period;
        // 除去已消耗的原始时钟周期
        self.n %= self.period;
        return rs;
    }
}

