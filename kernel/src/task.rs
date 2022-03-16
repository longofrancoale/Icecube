use crate::cpu::to_usermode;
use crate::interrupts::Registers;
use crate::mm::{PhysMem, VirtAddr};
use crate::paging;
use crate::paging::{PageTable, PAGE_NX, PAGE_USER};
use core::alloc::Layout;
use xmas_elf::sections::ShType;

#[derive(Default)]
pub struct Context {
    pub regs: Registers,
    pub rip: usize,
    pub rsp: usize,
}

pub struct Task {
    context: Context,
    page_table: PageTable,
}

impl Task {
    pub fn new(allocator: &mut dyn PhysMem, mut page_table: PageTable) -> Option<Task> {
        const STACK_BASE: usize = 0xcafebabe00000000;
        let stack_phys = allocator.alloc_phys_zeroed(Layout::from_size_align(4096, 4096).ok()?)?;

        unsafe {
            page_table.map_raw(
                allocator,
                VirtAddr(STACK_BASE),
                paging::PageType::Page4K,
                stack_phys.0 | PAGE_NX | PAGE_USER | 3,
                true,
                true,
                true,
            )?;
        }

        Some(Task {
            context: Context {
                rsp: STACK_BASE + 4096,
                ..Default::default()
            },
            page_table,
        })
    }

    pub fn load_elf(&mut self, allocator: &mut dyn PhysMem, elf: &[u8]) -> Option<()> {
        let user_elf = xmas_elf::ElfFile::new(elf).unwrap();
        self.context.rip = user_elf.header.pt2.entry_point() as usize;

        for section in user_elf.section_iter() {
            if section.get_type().unwrap() != ShType::ProgBits {
                continue;
            }

            let base = section.address();
            let data = section.raw_data(&user_elf);

            let pages = data.len() / 4096 + 1;
            for page in 0..pages {
                let phys =
                    allocator.alloc_phys_zeroed(Layout::from_size_align(4096, 4096).ok()?)?;
                log::info!(
                    "Mapping {:#x} to {:#x}",
                    phys.0,
                    base as usize + (page * 4096)
                );

                unsafe {
                    self.page_table.map_raw(
                        allocator,
                        VirtAddr(base as usize + (page * 4096)),
                        paging::PageType::Page4K,
                        (phys.0 + (page * 4096)) | PAGE_USER | 3,
                        true,
                        true,
                        true,
                    )?;
                }
            }

            {
                let kernel_page_table = core!().kernel_page_table.lock();

                unsafe {
                    self.page_table.write_to_as_slice(
                        base as *mut u8,
                        data,
                        kernel_page_table.as_ref(),
                    )
                };

                log::debug!("{:#?}", kernel_page_table.as_ref());
            }
        }

        Some(())
    }

    pub fn run(&self) -> ! {
        log::info!("Jumping to {:#x} in user-mode", self.context.rip);
        unsafe {
            self.page_table.switch_to();
            to_usermode(self.context.rip, self.context.rsp)
        }
    }
}
