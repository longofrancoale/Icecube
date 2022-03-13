use core::alloc::Layout;
use core::mem::size_of;

use crate::mm::{PhysAddr, PhysMem, VirtAddr};

pub const PAGE_PRESENT: usize = 1 << 0;
pub const PAGE_WRITE: usize = 1 << 1;
pub const PAGE_USER: usize = 1 << 2;
pub const PAGE_NX: usize = 1 << 63;

#[repr(usize)]
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum PageType {
    Page4K = 4096,
    Page2M = 2 * 1024 * 1024,
    Page1G = 1 * 1024 * 1024 * 1024,
}

pub struct PageTable {
    table: PhysAddr,
}

impl PageTable {
    pub fn new(pmem: &mut dyn PhysMem) -> Option<PageTable> {
        let table = pmem.alloc_phys_zeroed(Layout::from_size_align(4096, 4096).ok()?)?;
        Some(PageTable { table })
    }

    pub unsafe fn switch_to(&self) {
        crate::cpu::set_cr3(self.table.0);
    }

    pub unsafe fn map_raw(
        &mut self,
        phys_mem: &mut dyn PhysMem,
        vaddr: VirtAddr,
        page_type: PageType,
        raw: usize,
        add: bool,
        update: bool,
        invlpg_on_update: bool,
    ) -> Option<()> {
        let mut indicies = [0; 4];
        let indicies = match page_type {
            PageType::Page4K => {
                indicies[0] = (vaddr.0 >> 39) & 0x1ff;
                indicies[1] = (vaddr.0 >> 30) & 0x1ff;
                indicies[2] = (vaddr.0 >> 21) & 0x1ff;
                indicies[3] = (vaddr.0 >> 12) & 0x1ff;
                &indicies[..4]
            }
            PageType::Page2M => {
                indicies[0] = (vaddr.0 >> 39) & 0x1ff;
                indicies[1] = (vaddr.0 >> 30) & 0x1ff;
                indicies[2] = (vaddr.0 >> 21) & 0x1ff;
                &indicies[..3]
            }
            PageType::Page1G => {
                indicies[0] = (vaddr.0 >> 39) & 0x1ff;
                indicies[1] = (vaddr.0 >> 30) & 0x1ff;
                &indicies[..2]
            }
        };

        let mut table = self.table;
        for (depth, &index) in indicies.iter().enumerate() {
            let ptp = PhysAddr(table.0 + index * size_of::<usize>());
            let vad = phys_mem.translate(ptp, size_of::<usize>())?;

            let mut ent = *(vad as *const usize);

            if depth != indicies.len() - 1 && (ent & PAGE_PRESENT) == 0 {
                if !add {
                    return None;
                }

                let new_table =
                    phys_mem.alloc_phys_zeroed(Layout::from_size_align(4096, 4096).ok()?)?;

                ent = new_table.0 | PAGE_USER | PAGE_WRITE | PAGE_PRESENT;
                *(vad as *mut usize) = ent;
            }

            if depth == indicies.len() - 1 && ((ent & PAGE_PRESENT) == 0 || update) {
                if (ent & PAGE_PRESENT) != 0 {
                    *(vad as *mut usize) = raw;

                    if invlpg_on_update && size_of::<VirtAddr>() == size_of::<usize>() {
                        crate::cpu::invlpg(vaddr.0);
                    }
                } else {
                    *(vad as *mut usize) = raw;
                }

                return Some(());
            } else if depth == indicies.len() - 1 {
                return None;
            }

            table = PhysAddr(ent & 0xffffffffff000);
        }

        unreachable!();
    }
}
