use crate::core::convention::{SCREEN_H, SCREEN_W};

#[repr(C)]
pub struct WindowConfig {
    /// Scaling how many times based on original size
    pub scale_factor: f32,
}

pub struct Window {
    /// window width
    pub width: u32,
    /// window height
    pub height: u32,
    /// Scaling how many times based on original size
    pub scale_factor: f32,
    pub buffer: Vec<u32>,
}

impl Window {
    pub fn create(config: &WindowConfig) -> Window {
        let width = (f32::from(SCREEN_W) * config.scale_factor).round() as u32;
        let height = (f32::from(SCREEN_H) * config.scale_factor).round() as u32;
        let buf_size = (width * height) as usize;

        Self {
            width,
            height,
            scale_factor: config.scale_factor,
            buffer: vec![0; buf_size],
        }
    }

    pub fn update_buffer(&mut self, o_buffer: &[u32]) {
        let buf_size = (self.width * self.height) as usize;
        let factor = self.scale_factor.round() as u32;
        for i in 0..buf_size {
            // Calculate origin index according to scaled index
            let oi = Self::_map_scaled_index(i, self.width, factor);
            self.buffer[i] = o_buffer[oi];
        }
    }

    fn _map_scaled_index(index: usize, width: u32, scale_factor: u32) -> usize {
        let x = index as u32 % width;
        let y = index as u32 / width;
        return (y / scale_factor * SCREEN_W as u32 + (x / scale_factor)) as usize;
    }
}