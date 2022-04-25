use core::alloc::GlobalAlloc;
use core::alloc::Layout;

use stivale_boot::v2::{StivaleMemoryMapEntryType, StivaleStruct};

use crate::{
    rangeset::{Range, RangeSet},
    sync::LockCell,
};

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(pub usize);

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub usize);

pub trait PhysMem {
    unsafe fn translate(&mut self, phys: PhysAddr, size: usize) -> Option<*mut u8>;

    fn alloc_phys(&mut self, layout: Layout) -> Option<PhysAddr>;

    fn alloc_phys_zeroed(&mut self, layout: Layout) -> Option<PhysAddr> {
        let alc = self.alloc_phys(layout)?;

        unsafe {
            let bytes = self.translate(alc, layout.size())?;
            core::ptr::write_bytes(bytes, 0, layout.size());
        }

        Some(alc)
    }
}

pub struct PhysicalMemory;

impl PhysicalMemory {
    pub fn alloc<T>(&mut self) -> Option<&'static mut T> {
        let addr = self.alloc_phys_zeroed(
            Layout::from_size_align(core::mem::size_of::<T>(), core::mem::align_of::<T>()).ok()?,
        )?;

        let virt = unsafe { self.translate(addr, core::mem::size_of::<T>()) }?;

        unsafe { Some(&mut *(virt as *mut T)) }
    }
}

impl PhysMem for PhysicalMemory {
    unsafe fn translate(&mut self, phys: PhysAddr, size: usize) -> Option<*mut u8> {
        if size == 0 {
            return None;
        }
        Some(phys.0 as *mut u8)
    }

    fn alloc_phys(&mut self, layout: Layout) -> Option<PhysAddr> {
        let mut phys_mem = ALLOCATOR.lock();
        phys_mem
            .as_mut()
            .map(|alloc| {
                alloc
                    .allocate(layout.size() as u64, layout.align() as u64)
                    .map(|addr| PhysAddr(addr))
            })
            .unwrap_or(None)
    }
}

pub static ALLOCATOR: LockCell<Option<RangeSet>> = LockCell::new(None);

#[global_allocator]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator;

struct GlobalAllocator;

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Get access to physical memory
        let mut pmem = ALLOCATOR.lock();
        pmem.as_mut()
            .and_then(|x| x.allocate(layout.size() as u64, layout.align() as u64))
            .unwrap_or(0) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Get access to physical memory
        let mut pmem = ALLOCATOR.lock();
        pmem.as_mut()
            .and_then(|x| {
                let end = (ptr as u64).checked_add(layout.size().checked_sub(1)? as u64)?;
                x.insert(Range {
                    start: ptr as u64,
                    end: end,
                });
                Some(())
            })
            .expect("Cannot free memory without initialized MM");
    }
}

pub fn init(boot_info: &'static StivaleStruct) -> Option<()> {
    log::info!("Bios Provided E820 Memory Map:");
    let mut mem = RangeSet::new();

    let mmap = boot_info.memory_map()?;
    for entry in mmap.iter() {
        log::info!(
            "BIOS-e820: [mem {:#016x}-{:#016x}] {:?}",
            entry.base,
            entry.end_address(),
            entry.entry_type()
        );

        if entry.entry_type() == StivaleMemoryMapEntryType::Usable {
            mem.insert(Range {
                start: entry.base,
                end: entry.end_address(),
            });
        }
    }

    let mut allocator = ALLOCATOR.lock();
    *allocator = Some(mem);

    Some(())
}

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!(
        "Allocation error with size {} and layout {}",
        layout.size(),
        layout.align()
    )
}
