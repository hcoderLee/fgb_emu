use std::cell::RefCell;
use crate::core::intf::{Intf, INTFlag};
use std::rc::Rc;
use crate::core::memory::Memory;

/// 为每个手柄按键分配一个u8类型的值，方向键在低4位，标准按键在高4位
#[derive(Clone)]
pub enum JoypadKey {
    Right = 0b0000_0001,
    Left = 0b0000_0010,
    Up = 0b0000_0100,
    Down = 0b000_1000,
    A = 0b0001_0000,
    B = 0b0010_0000,
    Select = 0b0100_0000,
    Start = 0b1000_0000,
}

/// 手柄一共有8个按键:
/// 4个方向键：上，下，左，右
/// 4个标准按钮: A, B, Select, Start
pub struct Joypad {
    /// 用于触发手柄中断事件
    intf: Rc<RefCell<Intf>>,
    /// 记录当前按下的键, 每一位对应一个按键的状态，0表示按下，1表示未按下
    signals: u8,
    /// 用于控制当前按下的键是方向键还是A，B，Select，Start键
    select: u8,
}

impl Joypad {
    pub fn power_up(intf: Rc<RefCell<Intf>>) -> Self {
        Self {
            intf,
            signals: 0xff, // 默认按键状态为都没有按下
            select: 0x00,
        }
    }

    /// 当按下某个按键
    pub fn keydown(&mut self, key: JoypadKey) {
        // 将按下的键位置0，代表按下该键
        self.signals &= !(key as u8);
        // 触发手柄中断
        self.intf.borrow_mut().hi(INTFlag::Joypad);
    }

    /// 松开某个按键
    pub fn keyup(&mut self, key: JoypadKey) {
        // 将松开的按键置1
        self.signals |= key as u8;
    }
}

/// 内存地址0xff00用于记录按下的键
/// Bit 7 ~ Bit 6: 未使用
/// Bit 5: 为0则表示低4位按下的是标准按键（A, B, Start, Select）
/// Bit 4: 为0则表示低4位按下的是方向键
/// Bit 3: 为0则表示按下的是Down或Start键，只读
/// Bit 2: 为0则表示按下的是Up或Select键，只读
/// Bit 1: 为0则表示按下的是Left或B键，只读
/// Bit 0: 为0则表示按下的是Right或A键，只读
impl Memory for Joypad {
    fn get(&self, a: u16) -> u8 {
        // 手柄事件相关的内存地址只有0xff00
        assert_eq!(a, 0xff00);
        if (self.select & 0b0001_0000) == 0x00 {
            // 按下的是方向键，取signals的低4位
            return self.select | self.signals & 0x0f;
        }
        if (self.select & 0b0010_0000) == 0x00 {
            // 按下的是标准按键，取signals的高4位
            return self.select | self.signals >> 4;
        }
        self.select
    }

    fn set(&mut self, a: u16, v: u8) {
        // 手柄事件相关的内存地址只有0xff00
        assert_eq!(a, 0xff00);
        self.select = v;
    }
}