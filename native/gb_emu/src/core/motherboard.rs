use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use crate::core::memory::Memory;
use crate::core::mmunit::MMUnit;
use crate::core::rtc::RTC;

// 主板，cup与MMU交互，MMU负责管理硬件外设
pub struct MotherBoard {
    pub mmu: Rc<RefCell<MMUnit>>,
    pub rtc: RTC,
}

impl MotherBoard {
    pub fn power_up<T: AsRef<Path>>(path: T, save_path: T) -> Self {
        let mmu = Rc::new(RefCell::new(MMUnit::power_up(path, save_path)));
        let rtc = RTC::power_up(mmu.borrow().term, mmu.clone());
        Self { mmu, rtc }
    }

    pub fn next(&mut self) -> u32 {
        if self.mmu.borrow().get(self.rtc.cpu.reg.pc) == 0x10 {
            self.mmu.borrow_mut().speed.switch_speed();
        }
        let cycles = self.rtc.next();
        self.mmu.borrow_mut().next(cycles);
        cycles
    }

    pub fn check_and_reset_gpu_updated(&mut self) -> bool {
        let is_vblank = self.mmu.borrow().gpu.v_blank;
        self.mmu.borrow_mut().gpu.v_blank = false;
        is_vblank
    }
}
