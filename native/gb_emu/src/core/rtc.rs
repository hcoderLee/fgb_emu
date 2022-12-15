use crate::core::cpu::Cpu;
use std::time::Duration;
use crate::core::convention::Term;
use std::rc::Rc;
use std::cell::RefCell;
use crate::core::memory::Memory;
use std::{thread, time};

// gb的cpu时钟频率
pub const CLOCK_FREQUENCY: u32 = 4_194_304;
// 在此时间段内累计执行的时钟周期（单位是ms），16ms也是渲染一帧所需的时间（假设帧率为60fps）
pub const STEP_TIME: u32 = 16;
// 规定每段时间内最多执行的时钟周期
pub const STEP_CYCLES: u32 = ((CLOCK_FREQUENCY as f64 / 1000f64) * STEP_TIME as f64) as u32;

pub struct RTC {
    pub cpu: Cpu,
    // 累计已执行的时钟周期，超出指定范围时重新计数
    step_cycles: u32,
    // 最近一次开始累计已执行的时钟周期
    step_zero: time::Instant,
    // 是否已重新累计执行的时钟周期
    step_flip: bool,
}

impl RTC {
    pub fn power_up(term: Term, mem: Rc<RefCell<dyn Memory>>) -> Self {
        let cpu = Cpu::power_up(term, mem);
        Self {
            cpu,
            step_cycles: 0,
            step_zero: time::Instant::now(),
            step_flip: false,
        }
    }

    // 现代CPU的频率要远大于gb，需要降低cpu执行指令的速度，使其与gb的cpu时钟频率一致
    // 这里我们采用在每段固定的时间内执行特定数量的指令，使得每秒执行的指令数量与gb一致
    pub fn next(&mut self) -> u32 {
        if self.step_cycles > STEP_CYCLES {
            // 规定时间段内执行的时钟周期达到上限
            self.step_flip = true;
            self.step_cycles -= STEP_CYCLES;
            let now = time::Instant::now();
            // 距离开始累计执行时钟周期过了多久
            let d = now.duration_since(self.step_zero);
            // 距离规定时间段结束还要多久
            let s = u64::from(STEP_TIME.saturating_sub(d.as_millis() as u32));
            // CPU休眠到下个规定的时间段
            thread::sleep(Duration::from_millis(s));
            // 重置开始累计执行时钟周期的时间
            self.step_zero = self.step_zero.checked_add(
                Duration::from_millis(u64::from(STEP_TIME))
            ).unwrap();

            // 正常情况下，此时的step_zero要在now之后，但是sleep函数通常会比设定的时间睡眠的更久，累计的误差可能会
            // 使now在step_zero之后，当出现这种情况时要将step_zero设定为now，清空sleep导致的误差
            if now.checked_duration_since(self.step_zero).is_some() {
                self.step_zero = now;
            }
        }
        // 累计cpu执行下一条指令花费的时钟周期
        let cycles = self.cpu.next();
        self.step_cycles += cycles;
        cycles
    }

    // 用于判断是否产生了新的一帧
    pub fn flip(&mut self) -> bool {
        let r = self.step_flip;
        if r {
            self.step_flip = false;
        }
        r
    }
}