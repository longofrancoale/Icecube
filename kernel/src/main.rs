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

use stivale_boot::v2::{
    StivaleFramebufferHeaderTag, StivaleHeader, StivalePmrPermissionFlags, StivaleStruct,
};

use crate::{
    mm::{DumbPhysMem, PhysAddr, VirtAddr},
    paging::{PageTable, PAGE_NX, PAGE_PRESENT, PAGE_WRITE},
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

macro_rules! write {
    ($to:ident, $($arg:tt)*) => {
        let _ = $to.write_fmt(format_args!($($arg)*));
    };
}

#[no_mangle]
extern "C" fn _start(boot_info: &'static StivaleStruct) -> ! {
    let mut allocator = DumbPhysMem::new(PhysAddr(2 * 1024 * 1024));
    core_locals::init(&mut allocator);

    let mut page_table = PageTable::new(&mut allocator).unwrap();

    let mut serial = EmergencySerial;

    let kernel_base = boot_info.kernel_base_addr().unwrap();
    let kernel_phys_base = PhysAddr(kernel_base.physical_base_address as usize);
    let kernel_virt_base = VirtAddr(kernel_base.virtual_base_address as usize);

    for pmr in boot_info.pmrs().unwrap().as_slice() {
        let phys_base = PhysAddr(kernel_phys_base.0 + (pmr.base as usize - kernel_virt_base.0));
        let virt_base = VirtAddr(pmr.base as usize);

        let write = pmr
            .permissions()
            .contains(StivalePmrPermissionFlags::WRITABLE);
        let exec = pmr
            .permissions()
            .contains(StivalePmrPermissionFlags::EXECUTABLE);

        let flags =
            PAGE_PRESENT | if write { PAGE_WRITE } else { 0 } | if exec { 0 } else { PAGE_NX };

        write!(
            serial,
            "Mapping {:x?} to {:x?}, flags: {:?}, for {:#x}b({:#x} pages)\n",
            phys_base,
            virt_base,
            pmr.permissions(),
            pmr.size,
            pmr.size / 0x1000
        );
        for i in 0..(pmr.size as usize / 0x1000) {
            unsafe {
                page_table
                    .map_raw(
                        &mut allocator,
                        VirtAddr(virt_base.0 + i * 4096),
                        paging::PageType::Page4K,
                        (phys_base.0 + (i * 4096)) | flags,
                        true,
                        true,
                        true,
                    )
                    .unwrap();
            }
        }
    }

    for paddr in (0..(1 * 1024 * 1024 * 1024)).step_by(4096) {
        if unsafe {
            page_table.map_raw(
                &mut allocator,
                VirtAddr(paddr),
                paging::PageType::Page4K,
                paddr | 3,
                true,
                true,
                false,
            )
        }
        .is_none()
        {
            panic!("Update {:#x}?", paddr);
        }
    }

    unsafe { page_table.switch_to() }

    write!(serial, "Hello, from OUR paged memory!\n");

    loop {}
}
