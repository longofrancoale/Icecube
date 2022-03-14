#![allow(dead_code)]

// MSR for active GS base
pub const IA32_GS_BASE: u32 = 0xc0000101;

#[inline]
pub unsafe fn wrmsr(msr: u32, val: u64) {
    let low = (val & 0xFFFFFFFF) as u32;
    let high = (val >> 32) as u32;
    core::arch::asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high);
}

#[inline]
pub unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    core::arch::asm!("rdmsr", in("ecx") msr, out("eax") low, out("edx") high);
    ((high as u64) << 32) | low as u64
}

#[inline]
pub unsafe fn set_gs_base(base: u64) {
    wrmsr(IA32_GS_BASE, base);
}

#[inline]
pub unsafe fn invlpg(page: usize) {
    core::arch::asm!("invlpg [{}]", in(reg) page);
}

#[inline]
pub unsafe fn set_cr3(new_cr3: usize) {
    core::arch::asm!("mov cr3, {}", in(reg) new_cr3);
}

#[inline]
pub unsafe fn rdtsc() -> u64 {
    core::arch::x86_64::_rdtsc()
}

#[inline]
pub unsafe fn to_usermode(at: usize, sp: usize) -> ! {
    core::arch::asm!(r#"
        mov ax, 0x20 | 3
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax

        push 0x20 | 3
        push {stack}
        pushfq
        push 0x18 | 3
        push {addr}
        iretq
    "#, addr = in(reg) at, stack = in(reg) sp, options(noreturn));
}
