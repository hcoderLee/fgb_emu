use std::ffi::CStr;
use std::os::raw::c_char;
use crate::device::emulator::Emulator;
use crate::device::keyboard::GbBtn;
use crate::device::window::WindowConfig;
use std::thread::{self, JoinHandle};

static mut RUNNING_EMU: Option<JoinHandle<()>> = None;

#[no_mangle]
pub extern "C" fn create_emulator(win_config: *const WindowConfig) -> *mut Emulator {
    let win_config = unsafe { &*win_config };
    let emulator = Box::new(Emulator::create(win_config));
    return Box::into_raw(emulator);
}

#[no_mangle]
pub extern "C" fn run_emulator(emulator: *mut Emulator, rom_path: *const c_char) {
    unsafe {
        let c_str = CStr::from_ptr(rom_path);
        let path = c_str.to_str().unwrap();
        let emulator = &mut *emulator;
        if emulator.is_running() {
            return;
        }

        RUNNING_EMU = Some(thread::spawn(move || {
            emulator.run(path);
        }));
    }
}

#[no_mangle]
pub extern "C" fn get_window_buffer(emulator: *mut Emulator) -> *const u32 {
    let emulator = unsafe { &mut *emulator };
    emulator.get_window_buffer().as_ptr()
}

#[no_mangle]
pub extern "C" fn press_button(emulator: *mut Emulator, btn: GbBtn) {
    let emulator = unsafe { &mut *emulator };
    emulator.press_button(btn);
}

#[no_mangle]
pub extern "C" fn release_button(emulator: *mut Emulator, btn: GbBtn) {
    let emulator = unsafe { &mut *emulator };
    emulator.release_button(btn);
}

#[no_mangle]
pub extern "C" fn pause_emulator(emulator: *mut Emulator) {
    let emulator = unsafe { &mut *emulator };
    emulator.pause();
}

#[no_mangle]
pub extern "C" fn resume_emulator(emulator: *mut Emulator) {
    unsafe {
        if let Some(running_thd) = &RUNNING_EMU {
            let emulator = &mut *emulator;
            emulator.resume(running_thd.thread());
        }
    }
}

#[no_mangle]
pub extern "C" fn exit_emulator(emulator: *mut Emulator) {
    unsafe {
        (&mut *emulator).exit();
        // Release emulator object which created by [create_emulator]
        let _ = Box::from_raw(emulator);
        RUNNING_EMU = None;
    }
}