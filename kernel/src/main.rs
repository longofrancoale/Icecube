#![no_std]
#![no_main]
#![feature(panic_info_message)]

#[macro_use]
mod core_locals;
mod cpu;
mod mm;
mod paging;
mod panic;
mod serial;

use core::fmt::Write;

use stivale_boot::v2::{StivaleFramebufferHeaderTag, StivaleHeader, StivaleStruct};

use crate::{
    mm::{DumbPhysMem, PhysAddr},
    serial::EmergencySerial,
};

static STACK: [u8; 4096] = [0; 4096];

static FRAMEBUFFER_TAG: StivaleFramebufferHeaderTag =
    StivaleFramebufferHeaderTag::new().framebuffer_bpp(24);

#[link_section = ".stivale2hdr"]
#[no_mangle]
#[used]
static STIVALE_HDR: StivaleHeader = StivaleHeader::new()
    .stack(&STACK[4095] as *const u8)
    .tags((&FRAMEBUFFER_TAG as *const StivaleFramebufferHeaderTag).cast())
    .flags(0xF);

#[no_mangle]
extern "C" fn _start(boot_info: &'static StivaleStruct) -> ! {
    let mut allocator = DumbPhysMem::new(PhysAddr(2 * 1024 * 1024));
    core_locals::init(&mut allocator);

    let mut serial = EmergencySerial;
    serial.write_fmt(format_args!(
        "{:#x?}\n",
        boot_info.kernel_slide().map(|k| { k.kernel_slide })
    ));

    panic!("Moose {}", core!().id);
}
