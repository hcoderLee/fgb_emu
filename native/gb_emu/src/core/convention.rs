#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Term {
    /// 原版的GameBoy
    GB,
    /// GameBoy Pocket/GameBoy Light
    GBP,
    /// GameBoy Color
    GBC,
    /// Super GameBoy
    SGB,
}

/// CPU频率
pub const CPU_FREQ: u32 = 4194304;

///  屏幕的宽高
pub const SCREEN_W: u8 = 160;
pub const SCREEN_H: u8 = 144;

