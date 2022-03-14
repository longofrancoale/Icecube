#![no_std]
#![no_main]
#![feature(panic_info_message)]

#[macro_use]
mod core_locals;
mod cpu;
mod interrupts;
mod logging;
mod mm;
mod paging;
mod panic;
mod serial;

use core::alloc::Layout;

use logging::Logger;
use mm::PhysMem;
use paging::PAGE_USER;
use stivale_boot::v2::{
    StivaleFramebufferHeaderTag, StivaleHeader, StivalePmrPermissionFlags, StivaleStruct,
};
use xmas_elf::sections::ShType;

use crate::{
    mm::{DumbPhysMem, PhysAddr, VirtAddr},
    paging::{PageTable, PAGE_NX, PAGE_PRESENT, PAGE_WRITE},
};

pub static STACK: [u8; 32 * 1024] = [0; 32 * 1024];

static FRAMEBUFFER_TAG: StivaleFramebufferHeaderTag =
    StivaleFramebufferHeaderTag::new().framebuffer_bpp(24);

static LOGGER: Logger = Logger;

#[link_section = ".stivale2hdr"]
#[no_mangle]
#[used]
static STIVALE_HDR: StivaleHeader = StivaleHeader::new()
    .stack(&STACK[4095] as *const u8)
    .tags((&FRAMEBUFFER_TAG as *const StivaleFramebufferHeaderTag).cast())
    .flags(0xF);

#[no_mangle]
extern "C" fn _start(boot_info: &'static StivaleStruct) -> ! {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    let mut allocator = DumbPhysMem::new(PhysAddr(2 * 1024 * 1024));
    core_locals::init(&mut allocator);

    let mut page_table = PageTable::new(&mut allocator).unwrap();

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

    for paddr in (0..(4 * 1024 * 1024 * 1024)).step_by(4096) {
        unsafe {
            page_table
                .map_raw(
                    &mut allocator,
                    VirtAddr(paddr),
                    paging::PageType::Page4K,
                    paddr | 3,
                    true,
                    true,
                    false,
                )
                .unwrap()
        }
    }

    unsafe { page_table.switch_to() }

    interrupts::init(&mut allocator);

    let user_stack_phys = allocator
        .alloc_phys_zeroed(Layout::from_size_align(4096, 4096).unwrap())
        .unwrap();
    log::info!("User stack at {:x?}", user_stack_phys);
    let user_stack_virt = 0xcafebabe00000000usize;

    unsafe {
        page_table
            .map_raw(
                &mut allocator,
                VirtAddr(user_stack_virt),
                paging::PageType::Page4K,
                user_stack_phys.0 | PAGE_NX | PAGE_USER | 3,
                true,
                true,
                true,
            )
            .unwrap()
    };

    static USER_ELF: &[u8] = include_bytes!("../user-test/main");
    let user_elf = xmas_elf::ElfFile::new(USER_ELF).unwrap();
    let entry = user_elf.header.pt2.entry_point() as usize;

    for section in user_elf.section_iter() {
        if section.get_type().unwrap() != ShType::ProgBits {
            continue;
        }

        let base = section.address();
        let data = section.raw_data(&user_elf);

        let pages = data.len() / 4096 + 1;
        for page in 0..pages {
            let phys = allocator
                .alloc_phys_zeroed(Layout::from_size_align(4096, 4096).unwrap())
                .unwrap();
            log::info!(
                "Mapping {:#x} to {:#x}",
                phys.0,
                base as usize + (page * 4096)
            );

            unsafe {
                page_table
                    .map_raw(
                        &mut allocator,
                        VirtAddr(base as usize + (page * 4096)),
                        paging::PageType::Page4K,
                        (phys.0 + (page * 4096)) | PAGE_USER | 3,
                        true,
                        true,
                        false,
                    )
                    .unwrap()
            }

            let ptr = unsafe {
                core::slice::from_raw_parts_mut((base as usize + (page * 4096)) as *mut u8, 4096)
            };
            ptr[0..data.len()].copy_from_slice(data);
        }
    }

    log::info!("Entry at {:#x}", entry);

    unsafe { cpu::to_usermode(entry, user_stack_virt) }
}
