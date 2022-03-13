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
