use crate::core::convention::{SCREEN_H, SCREEN_W};
use crate::core::motherboard::MotherBoard;
use crate::device::keyboard::{GbBtn, Keyboard, KEY_MAPS};
use crate::device::window::{Window, WindowConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::Thread;

pub struct Emulator {
    window: Window,
    keyboard: Keyboard,
    is_running: AtomicBool,
    is_pause: AtomicBool,
}

impl Emulator {
    pub fn create(win_config: &WindowConfig) -> Self {
        Self {
            window: Window::create(win_config),
            keyboard: Keyboard::create(),
            is_running: AtomicBool::new(false),
            is_pause: AtomicBool::new(false),
        }
    }

    // This method will called in new thread
    pub fn run(&mut self, rom_path: &str, save_path: &str) {
        if self.is_running.load(Ordering::Acquire) {
            log::warn!("{} is already running", rom_path);
            return;
        }

        log::info!("Running {}", rom_path);
        self.is_running.store(true, Ordering::Release);
        // 主板，用于管理cpu和各种外设
        let mut mbrd = MotherBoard::power_up(rom_path, save_path);
        // 初始化音频播放
        // initialize_audio(&mbrd);

        // 屏幕显示的像素数据，初始化为纯黑的背景
        let mut win_buf: Vec<u32> =
            vec![0x00; (u32::from(SCREEN_W) * u32::from(SCREEN_H)) as usize];

        // 设置第一帧画面
        self.window.update_buffer(&win_buf);

        loop {
            if !self.is_running.load(Ordering::Acquire) {
                break;
            }
            if self.is_pause.load(Ordering::Acquire) {
                thread::park();
            }

            // 执行一条指令
            mbrd.next();

            // 在发生vblank时刷新屏幕数据
            if mbrd.check_and_reset_gpu_updated() {
                // 刷新要显示的数据
                let mut i: usize = 0;
                for r in (*mbrd.mmu).borrow().gpu.data.iter() {
                    for c in r {
                        let b = u32::from(c[0]);
                        let g = u32::from(c[1]) << 8;
                        let r = u32::from(c[2]) << 16;
                        let a: u32 = 0xff00_0000;
                        win_buf[i] = a | r | g | b;
                        i += 1;
                    }
                }
                // 上屏
                self.window.update_buffer(&win_buf);
            }

            if !mbrd.rtc.flip() {
                continue;
            }

            // 处理手柄事件
            for (rk, vk) in KEY_MAPS {
                if self.keyboard.is_button_pressed(rk) {
                    mbrd.mmu.borrow_mut().joypad.keydown(vk);
                } else {
                    mbrd.mmu.borrow_mut().joypad.keyup(vk);
                }
            }
        }

        let cartridge = &mbrd.mmu.borrow().cartridge;
        log::info!("Save game {}", cartridge.title());
        // 保存游戏数据
        cartridge.save();
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Acquire)
    }

    pub fn pause(&mut self) {
        self.is_pause.store(true, Ordering::Release);
        log::info!("Pause emulator");
    }

    pub fn resume(&mut self, thread: &Thread) {
        self.is_pause.store(false, Ordering::Release);
        thread.unpark();
        log::info!("Resume emulator");
    }

    pub fn exit(&mut self) {
        self.is_running.store(false, Ordering::Release);
        log::info!("Exit emulator");
    }

    pub fn press_button(&mut self, btn: GbBtn) {
        self.keyboard.press_button(btn);
        log::info!("Press {} button", btn);
    }

    pub fn release_button(&mut self, btn: GbBtn) {
        self.keyboard.release_button(btn);
        log::info!("Release {} button", btn);
    }

    pub fn get_window_buffer(&self) -> &Vec<u32> {
        &self.window.buffer
    }
}
