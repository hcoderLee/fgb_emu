use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};

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
    win_buffer: WindowBuffer,
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
            win_buffer: WindowBuffer::new(buf_size),
        }
    }

    pub fn update_buffer(&mut self, o_buffer: &[u32]) {
        let mut row_iter = o_buffer.chunks(SCREEN_W as usize);
        let mut buffer = self.win_buffer.get_free_buffer();
        buffer.clear();
        while let Some(o_row) = row_iter.next() {
            let mut row = vec![0; self.width as usize];
            for i in 0..row.len() {
                row[i] = o_row[i / self.scale_factor as usize];
            }
            buffer.append(&mut row.repeat(self.scale_factor as usize));
        }
        self.win_buffer.add_render_buffer(buffer);
    }

    pub fn get_buffer(&mut self) -> &[u32] {
        self.win_buffer.get_render_buffer()
    }
}

/// A data structure that read and write window buffer, cause read and write happens in different
/// threads, so it uses spin lock to protect enqueue and dequeue operations
struct WindowBuffer {
    // Store the frames that ready to be rendered
    buffers: VecDeque<Vec<u32>>,
    // Store the free buffers
    caches: VecDeque<Vec<u32>>,
    // The size of each frame
    buf_size: usize,
    // Spin lock
    lock: AtomicBool,
}

impl WindowBuffer {
    fn new(size: usize) -> Self {
        Self {
            buffers: VecDeque::with_capacity(2),
            caches: VecDeque::with_capacity(2),
            buf_size: size,
            lock: AtomicBool::new(false),
        }
    }

    /// Spin to get lock
    fn acquire_lock(&self) {
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {}
    }

    /// Spin to release lock
    fn release_lock(&self) {
        while self
            .lock
            .compare_exchange_weak(true, false, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {}
    }

    /// Get a cached frame which can write new data to it
    pub fn get_free_buffer(&mut self) -> Vec<u32> {
        self.acquire_lock();
        // Dequeue a buffer from cache
        let buffer = match self.caches.pop_front() {
            None => {
                // If cache is empty, create a new frame buffer
                Vec::with_capacity(self.buf_size)
            }
            Some(buf) => buf,
        };
        self.release_lock();
        return buffer;
    }

    /// Enqueue a frame to buffers, wait for rendering
    pub fn add_render_buffer(&mut self, buffer: Vec<u32>) {
        self.acquire_lock();
        if self.buffers.len() > 1 {
            // Move the last frame to caches (First frame is rendering now)
            let old_buffer = self.buffers.pop_back().unwrap();
            self.caches.push_back(old_buffer);
        }
        // We just ignore the old frame (moved to caches in last step) which was not rendered, and
        // enqueue the latest frame
        self.buffers.push_back(buffer);
        self.release_lock();
    }

    /// Get the latest frame which ready to be rendered
    pub fn get_render_buffer(&mut self) -> &[u32] {
        self.acquire_lock();
        if self.buffers.len() > 1 {
            // Dequeue last rendered frame (the first frame) if next frame is ready
            let old_buf = self.buffers.pop_front().unwrap();
            // Move it to caches
            self.caches.push_back(old_buf);
        }
        // Add an empty frame if there were no readied frames
        if self.buffers.is_empty() {
            self.buffers.push_back(vec![0; self.buf_size]);
        }
        // Return a fresh frame (the new first frame) for rendering
        let buffer = self.buffers.get(0).unwrap();
        self.release_lock();
        return buffer;
    }
}
