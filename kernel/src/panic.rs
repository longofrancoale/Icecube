use core::{fmt::Write, panic::PanicInfo};

use crate::serial::EmergencySerial;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut serial = EmergencySerial;

    let _ = serial.write_fmt(format_args!("PANIC: \n"));
    let _ = info.location().map(|loc| {
        serial.write_fmt(format_args!(
            "  At {} {}:{}\n",
            loc.file(),
            loc.line(),
            loc.column()
        ))
    });
    let _ = info
        .message()
        .map(|message| serial.write_fmt(format_args!("{}\n", message)));

    loop {}
}
