use log::{Level, LevelFilter, Log, Metadata, Record};
use yansi::Paint;

/// Custom log implementation, which will send log messages to flutter
pub struct FLogger {
    pub isolate: Option<allo_isolate::Isolate>,
}

impl Log for FLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        // Format log message
        let header = match record.level() {
            Level::Error => Paint::red("ERROR").bold(),
            Level::Warn => Paint::yellow("WARN").bold(),
            Level::Info => Paint::green("INFO").bold(),
            Level::Debug => Paint::blue("DEBUG").bold(),
            Level::Trace => Paint::magenta("TRACE").bold(),
        };
        let msg = format!("{} {} > {}", header, Paint::new(record.target()), record.args());
        // Send log message to flutter
        if let Some(isolate) = self.isolate {
            isolate.post(msg);
        }
    }

    fn flush(&self) {}
}

pub static mut LOGGER: FLogger = FLogger { isolate: None };

#[no_mangle]
pub extern "C" fn init_logger(port: i64, post_c_object: allo_isolate::ffi::DartPostCObjectFnType) {
    // Create Isolate instance to communicate with flutter
    let isolate = allo_isolate::Isolate::new(port);
    unsafe {
        allo_isolate::store_dart_post_cobject(post_c_object);
    }
    // Config logger
    let logger = unsafe { &mut LOGGER };
    logger.isolate = Some(isolate);
    // Set our custom log implementation
    log::set_logger(unsafe { &LOGGER }).expect("Log set logger failed");
    // Set max log level (All of log messages will be ignored if not set)
    log::set_max_level(LevelFilter::Trace);
}