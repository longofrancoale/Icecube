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
