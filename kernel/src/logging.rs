use core::fmt::Write;

use crate::serial::EmergencySerial;

pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let mut serial = EmergencySerial;

        let _ = serial.write_fmt(format_args!(
            "{}: {}-{} {}\n",
            record.level(),
            record.file().unwrap_or("<unknown file>"),
            record.line().unwrap_or(0),
            record.args()
        ));
    }

    fn flush(&self) {}
}
