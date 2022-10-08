use std::fs::File;
use std::io::Write;

pub struct Logger {
    log_file: Option<File>,
    count: u32,
}

impl Logger {
    pub fn power_up(file_name: &str, enable: bool) -> Self {
        Self {
            log_file: if enable { Some(File::create(file_name).unwrap()) } else { None },
            count: 0,
        }
    }

    pub fn i(&mut self, info: String) {
        if self.log_file.is_none() {
            return;
        }

        let is_record = self.condition();
        if let Some(f) = &mut self.log_file {
            if is_record {
                f.write_all(info.as_bytes()).unwrap();
            }
        }
    }

    fn condition(&mut self) -> bool {
        let is_log = self.count <= 10_0000;
        self.count += 1;
        return is_log;
    }
}