#![no_std]
#![no_main]
#![feature(panic_info_message, naked_functions, asm_sym)]

#[macro_use]
mod core_locals;
mod cpu;
mod interrupts;
mod logging;
mod mm;
mod paging;
mod panic;
mod rangeset;
mod serial;
mod sync;
mod task;

use crate::{
    interrupts::{InterruptFrame, Interrupts},
    mm::{PhysAddr, VirtAddr},
    paging::{PageTable, PAGE_NX, PAGE_PRESENT, PAGE_WRITE},
};
use interrupts::Registers;
use logging::Logger;
use mm::PhysMem;
use stivale_boot::v2::{
    StivaleFramebufferHeaderTag, StivaleHeader, StivalePmrPermissionFlags, StivaleStruct,
};

pub static STACK: [u8; 32 * 1024] = [0; 32 * 1024];

static FRAMEBUFFER_TAG: StivaleFramebufferHeaderTag =
    StivaleFramebufferHeaderTag::new().framebuffer_bpp(24);

static LOGGER: Logger = Logger;

#[link_section = ".stivale2hdr"]
#[no_mangle]
#[used]
static STIVALE_HDR: StivaleHeader = StivaleHeader::new()
    .stack(&STACK[STACK.len() - 1] as *const u8)
    .tags((&FRAMEBUFFER_TAG as *const StivaleFramebufferHeaderTag).cast())
    .flags(0xF);

pub fn new_kernel_pagetable(
    allocator: &mut dyn PhysMem,
    boot_info: &'static StivaleStruct,
) -> PageTable {
    let mut page_table = PageTable::new(allocator).unwrap();

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
                        allocator,
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
                    allocator,
                    VirtAddr(paddr),
                    paging::PageType::Page4K,
                    paddr | 3,
                    true,
                    true,
                    false,
                )
                .unwrap()
        }
        unsafe {
            page_table
                .map_raw(
                    allocator,
                    VirtAddr(paddr + 0xffff800000000000),
                    paging::PageType::Page4K,
                    paddr | 3,
                    true,
                    true,
                    false,
                )
                .unwrap()
        }
    }

    page_table
}

#[no_mangle]
extern "C" fn _start(boot_info: &'static StivaleStruct) -> ! {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    mm::init(boot_info).unwrap();

    core_locals::init(&mut mm::PhysicalMemory);

    let page_table = new_kernel_pagetable(&mut mm::PhysicalMemory, boot_info);

    unsafe { page_table.switch_to() }

    {
        let mut kernel_page_table = core!().kernel_page_table.lock();
        *kernel_page_table = Some(page_table);
    }

    {
        let kernel_page_table = core!().kernel_page_table.lock();
        log::debug!("{:#?}", kernel_page_table.as_ref());
    }

    {
        let mut interrupts = core!().interrupt_state.lock();
        *interrupts = Some(Interrupts::init(&mut mm::PhysicalMemory));
    }

    let user_page_table = new_kernel_pagetable(&mut mm::PhysicalMemory, boot_info);
    let mut user_task = task::Task::new(&mut mm::PhysicalMemory, user_page_table).unwrap();

    let mut init = None;

    let modules = boot_info.modules().unwrap();
    for module in modules.iter() {
        if module.as_str() == "__INIT__" {
            init = Some(unsafe {
                core::slice::from_raw_parts_mut(module.start as *mut u8, module.size() as usize)
            });
        }
    }

    user_task
        .load_elf(&mut mm::PhysicalMemory, init.unwrap())
        .unwrap();

    // TODO: Use the right thing
    unsafe {
        core::arch::asm!(r#"
        pushfq
        pop {rflags}

        or {rflags}, 1 << 12
        or {rflags}, 1 << 13

        push {rflags}
        popf
        "#, rflags = lateout(reg) _)
    };

    user_task.run()
}

pub extern "C" fn int80(frame: &InterruptFrame, regs: &Registers) {
    if frame.cs & 0x3 == 0x3 {
        unsafe { core::arch::asm!("swapgs") };
    }

    unsafe {
        let kernel_page_table = core!().kernel_page_table.lock();
        let kernel_page_table = kernel_page_table.as_ref().unwrap();
        kernel_page_table.switch_to();
    }

    log::info!("SYSCALL: \n{:#x?}", regs);

    loop {}
}
