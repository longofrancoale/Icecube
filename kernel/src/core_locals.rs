use core::{alloc::Layout, sync::atomic::AtomicUsize, sync::atomic::Ordering};

use alloc::vec;
use alloc::vec::Vec;

use crate::{interrupts::Interrupts, mm::PhysMem, paging::PageTable, sync::LockCell, task::Task};

static CORES_ONLINE: AtomicUsize = AtomicUsize::new(0);

#[repr(C)]
pub struct CoreLocals {
    address: usize,

    pub id: usize,

    pub kernel_page_table: LockCell<Option<PageTable>>,
    pub interrupt_state: LockCell<Option<Interrupts>>,
    pub tasks: LockCell<Vec<&'static mut Task>>,
    pub current_task_id: LockCell<usize>,
}

trait CoreGuard: Sync + Sized {}
impl CoreGuard for CoreLocals {}

#[macro_export]
macro_rules! core {
    () => {
        $crate::core_locals::get_core_locals()
    };
}

#[inline]
#[allow(dead_code)]
pub fn get_core_locals() -> &'static CoreLocals {
    unsafe {
        let ptr: usize;
        core::arch::asm!("mov {}, gs:[0]", out(reg) ptr);

        &*(ptr as *const CoreLocals)
    }
}

pub fn init(phys_mem: &mut dyn PhysMem) {
    let core_locals_ptr = phys_mem
        .alloc_phys_zeroed(
            Layout::from_size_align(
                core::mem::size_of::<CoreLocals>(),
                core::mem::align_of::<CoreLocals>(),
            )
            .unwrap(),
        )
        .unwrap()
        .0;

    let core_locals = CoreLocals {
        address: core_locals_ptr,
        id: CORES_ONLINE.fetch_add(1, Ordering::SeqCst),
        kernel_page_table: LockCell::new(None),
        interrupt_state: LockCell::new(None),
        tasks: LockCell::new(vec![]),
        current_task_id: LockCell::new(0),
    };

    unsafe {
        core::ptr::write(core_locals_ptr as *mut CoreLocals, core_locals);
        crate::cpu::set_kernel_gs_base(core_locals_ptr as u64);
        core::arch::asm!("swapgs");
    }
}
