use crate::mm::PhysMem;
use core::alloc::Layout;

pub fn init(allocator: &mut dyn PhysMem) {
    let gdt_page = allocator
        .alloc_phys_zeroed(Layout::from_size_align(4096, 4096).unwrap())
        .unwrap();

    let gdt = unsafe { core::slice::from_raw_parts_mut(gdt_page.0 as *mut u64, 5) };
    gdt[0] = 0;
    gdt[1] = 0x00209a0000000000; // 0x08 KC
    gdt[2] = 0x0000920000000000; // 0x10 KD
    gdt[3] = 0x0020fb0000000000; // 0x18 UC
    gdt[4] = 0x0000f30000000000; // 0x20 UD

    let mut gdtr = [0u8; 10];
    gdtr[0..2].copy_from_slice(&40u16.to_ne_bytes());
    gdtr[2..10].copy_from_slice(&(gdt.as_ptr() as u64).to_ne_bytes());

    unsafe {
        core::arch::asm!("lgdt [{}]", in(reg) &gdtr);
        core::arch::asm!(r#"
            mov ax, 0x10
            mov ds, ax
            mov es, ax
            mov fs, ax
            mov gs, ax
            mov ss, ax

            push 0x08
            lea {tmp}, [2f + rip]
            push {tmp}
            retfq

            2:
        "#, tmp = lateout(reg) _, lateout("ax") _);
    }
}
