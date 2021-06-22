macro_rules! debug {
    ($target:expr, $fmt:literal, $($arg:tt)*) => {
    	log::debug!("[{}]: {}", $target, format!($fmt, $($arg)*));
    };
    ($target:expr, $msg:expr) => {
        log::debug!("[{}]: {}", $target, $msg);
    };
}

macro_rules! error {
    ($target:expr, $fmt:literal, $($arg:tt)*) => {
    	log::error!("[{}]: {}", $target, format!($fmt, $($arg)*));
    };
    ($target:expr, $msg:expr) => {
        log::error!("[{}]: {}", $target, $msg);
    };
}

macro_rules! info {
    ($target:expr, $fmt:literal, $($arg:tt)*) => {
    	log::info!("[{}]: {}", $target, format!($fmt, $($arg)*));
    };
    ($target:expr, $msg:expr) => {
        log::info!("[{}]: {}", $target, $msg);
    };
}

macro_rules! trace {
    ($target:expr, $fmt:literal, $($arg:tt)*) => {
    	log::trace!("[{}]: {}", $target, format!($fmt, $($arg)*));
    };
    ($target:expr, $msg:expr) => {
        log::trace!("[{}]: {}", $target, $msg);
    };
}

macro_rules! warn {
    ($target:expr, $fmt:literal, $($arg:tt)*) => {
    	log::warn!("[{}]: {}", $target, format!($fmt, $($arg)*));
    };
    ($target:expr, $msg:expr) => {
        log::warn!("[{}]: {}", $target, $msg);
    };
}

#[cfg(test)]
mod test {
    use lazy_static::lazy_static;
    use simplelog::{Config, LevelFilter, WriteLogger};
    use std::io::Write;
    use std::ops::Deref;
    use std::sync::{Arc, Once, RwLock};

    #[derive(Clone, Default)]
    struct LogBuffer(Arc<RwLock<Vec<String>>>);

    impl Deref for LogBuffer {
        type Target = RwLock<Vec<String>>;
        fn deref(&self) -> &<Self as Deref>::Target {
            &self.0
        }
    }

    impl LogBuffer {
        pub fn new() -> Self {
            let b: Self = Default::default();
            {
                let mut w = b.write().unwrap();
                w.push(String::new());
            }
            b
        }

        pub fn writer(&self) -> LogBufferWriter {
            LogBufferWriter
        }

        pub fn find_msg(&self, msg: &str) -> Vec<String> {
            self.read()
                .unwrap()
                .iter()
                .filter(|ele| ele.contains(msg))
                .cloned()
                .collect()
        }
    }

    struct LogBufferWriter;

    impl Write for LogBufferWriter {
        fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
            // Push to latest member
            // If char == \n then make a new line
            let mut locked = LOG_BUFFER.write().unwrap();
            assert!(locked.len() >= 1);
            for chr in buf {
                if *chr != b'\n' {
                    let l = locked.len() - 1;
                    locked[l].push(*chr as char);
                } else {
                    locked.push(String::new());
                }
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> Result<(), std::io::Error> {
            Ok(())
        }
    }

    lazy_static! {
        static ref LOG_BUFFER: LogBuffer = LogBuffer::new();
    }

    static START: Once = Once::new();

    fn init() {
        START.call_once(|| {
            let buf = LOG_BUFFER.writer();
            let _ =
                WriteLogger::init(LevelFilter::Trace, Config::default(), buf);
        });
    }

    #[test]
    fn test_debug() {
        init();

        let t = "::1";

        debug!(t, "DEBUG MESSAGE 1");
        assert!(!LOG_BUFFER
            .find_msg(&format!("[{}]: DEBUG MESSAGE 1", t))
            .is_empty());

        let m = "HELLO THIS IS DEBUG MESSAGE 2";
        debug!(t, m);
        assert!(!LOG_BUFFER.find_msg(&format!("[{}]: {}", t, m)).is_empty());

        debug!("::1", "DEBUG MESSAGE 3");
        assert!(!LOG_BUFFER.find_msg("[::1]: DEBUG MESSAGE 3").is_empty());

        debug!(t, "DEBUG MESSAGE 4 WITH FMT ARGS {} {} {}", t, m, 5);
        assert!(!LOG_BUFFER
            .find_msg(&format!(
                "[{}]: DEBUG MESSAGE 4 WITH FMT ARGS {} {} {}",
                t, t, m, 5
            ))
            .is_empty());
    }

    #[test]
    fn test_error() {
        init();

        let t = "::1";

        error!(t, "ERROR MESSAGE 1");
        assert!(!LOG_BUFFER
            .find_msg(&format!("[{}]: ERROR MESSAGE 1", t))
            .is_empty());

        let m = "HELLO THIS IS ERROR MESSAGE 2";
        error!(t, m);
        assert!(!LOG_BUFFER.find_msg(&format!("[{}]: {}", t, m)).is_empty());

        error!("::1", "ERROR MESSAGE 3");
        assert!(!LOG_BUFFER.find_msg("[::1]: ERROR MESSAGE 3").is_empty());

        error!(t, "ERROR MESSAGE 4 WITH FMT ARGS {} {} {}", t, m, 5);
        assert!(!LOG_BUFFER
            .find_msg(&format!(
                "[{}]: ERROR MESSAGE 4 WITH FMT ARGS {} {} {}",
                t, t, m, 5
            ))
            .is_empty());
    }

    #[test]
    fn test_info() {
        init();

        let t = "::1";

        info!(t, "INFO MESSAGE 1");
        assert!(!LOG_BUFFER
            .find_msg(&format!("[{}]: INFO MESSAGE 1", t))
            .is_empty());

        let m = "HELLO THIS IS INFO MESSAGE 2";
        info!(t, m);
        assert!(!LOG_BUFFER.find_msg(&format!("[{}]: {}", t, m)).is_empty());

        info!("::1", "INFO MESSAGE 3");
        assert!(!LOG_BUFFER.find_msg("[::1]: INFO MESSAGE 3").is_empty());

        info!(t, "INFO MESSAGE 4 WITH FMT ARGS {} {} {}", t, m, 5);
        assert!(!LOG_BUFFER
            .find_msg(&format!(
                "[{}]: INFO MESSAGE 4 WITH FMT ARGS {} {} {}",
                t, t, m, 5
            ))
            .is_empty());
    }

    #[test]
    fn test_trace() {
        init();

        let t = "::1";

        trace!(t, "TRACE MESSAGE 1");
        assert!(!LOG_BUFFER
            .find_msg(&format!("[{}]: TRACE MESSAGE 1", t))
            .is_empty());

        let m = "HELLO THIS IS TRACE MESSAGE 2";
        trace!(t, m);
        assert!(!LOG_BUFFER.find_msg(&format!("[{}]: {}", t, m)).is_empty());

        trace!("::1", "TRACE MESSAGE 3");
        assert!(!LOG_BUFFER.find_msg("[::1]: TRACE MESSAGE 3").is_empty());

        trace!(t, "TRACE MESSAGE 4 WITH FMT ARGS {} {} {}", t, m, 5);
        assert!(!LOG_BUFFER
            .find_msg(&format!(
                "[{}]: TRACE MESSAGE 4 WITH FMT ARGS {} {} {}",
                t, t, m, 5
            ))
            .is_empty());
    }

    #[test]
    fn test_warn() {
        init();

        let t = "::1";

        warn!(t, "WARN MESSAGE 1");
        assert!(!LOG_BUFFER
            .find_msg(&format!("[{}]: WARN MESSAGE 1", t))
            .is_empty());

        let m = "HELLO THIS IS WARN MESSAGE 2";
        warn!(t, m);
        assert!(!LOG_BUFFER.find_msg(&format!("[{}]: {}", t, m)).is_empty());

        warn!("::1", "WARN MESSAGE 3");
        assert!(!LOG_BUFFER.find_msg("[::1]: WARN MESSAGE 3").is_empty());

        warn!(t, "WARN MESSAGE 4 WITH FMT ARGS {} {} {}", t, m, 5);
        assert!(!LOG_BUFFER
            .find_msg(&format!(
                "[{}]: WARN MESSAGE 4 WITH FMT ARGS {} {} {}",
                t, t, m, 5
            ))
            .is_empty());
    }
}
