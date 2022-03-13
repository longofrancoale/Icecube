use core::{alloc::Layout, sync::atomic::AtomicUsize, sync::atomic::Ordering};

use crate::mm::PhysMem;

static CORES_ONLINE: AtomicUsize = AtomicUsize::new(0);

#[repr(C)]
pub struct CoreLocals {
    address: usize,

    pub id: usize,
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
        .unwrap();

    let core_locals = CoreLocals {
        address: core_locals_ptr.0,
        id: CORES_ONLINE.fetch_add(1, Ordering::SeqCst),
    };

    unsafe {
        core::ptr::write(core_locals_ptr.0 as *mut CoreLocals, core_locals);
        crate::cpu::set_gs_base(core_locals_ptr.0 as u64);
    }
}
