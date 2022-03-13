use core::alloc::Layout;

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(pub usize);

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub usize);

pub trait PhysMem {
    fn translate(&mut self, phys: PhysAddr, size: usize) -> Option<*mut u8>;

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

pub struct DumbPhysMem(PhysAddr);

impl DumbPhysMem {
    pub fn new(next_addr: PhysAddr) -> DumbPhysMem {
        DumbPhysMem(next_addr)
    }
}

impl PhysMem for DumbPhysMem {
    fn translate(&mut self, phys: PhysAddr, _size: usize) -> Option<*mut u8> {
        Some(phys.0 as *mut u8)
    }

    fn alloc_phys(&mut self, layout: Layout) -> Option<PhysAddr> {
        let alc = self.0;
        self.0 = PhysAddr(self.0 .0 + (layout.size() + layout.align() - 1) & !(layout.size() - 1));
        Some(alc)
    }
}
