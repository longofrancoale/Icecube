#![allow(dead_code)]

use crate::mm::PhysMem;
use core::alloc::Layout;

#[derive(Debug)]
#[repr(C)]
pub struct InterruptFrame {
    pub rip: u64,
    cs: u64,
    rflags: u64,
    sp: u64,
    ss: u64,
}

#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Registers {
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rbp: u64,
    rsi: u64,
    rdi: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
}

macro_rules! handler {
    ($name: path) => {{
        #[naked]
        extern "C" fn wrapper() {
            unsafe {
                core::arch::asm!(
                    r#"
                    push r15
                    push r14
                    push r13
                    push r12
                    push r11
                    push r10
                    push r9
                    push r8
                    push rdi
                    push rsi
                    push rbp
                    push rdx
                    push rcx
                    push rbx
                    push rax

                    mov rdi, rsp
                    add rdi, 15*8
                    mov rsi, rsp
                    call {}

                    pop rax
                    pop rbx
                    pop rcx
                    pop rdx
                    pop rbp
                    pop rsi
                    pop rdi
                    pop r8
                    pop r9
                    pop r10
                    pop r11
                    pop r12
                    pop r13
                    pop r14
                    pop r15

                    iretq
                "#,
                    sym $name,
                    options(noreturn)
                )
            }
        }
        wrapper
    }};
}

macro_rules! handler_errorcode {
    ($name: path) => {{
        #[naked]
        extern "C" fn wrapper() {
            unsafe {
                core::arch::asm!(
                    r#"
                    push r15
                    push r14
                    push r13
                    push r12
                    push r11
                    push r10
                    push r9
                    push r8
                    push rdi
                    push rsi
                    push rbp
                    push rdx
                    push rcx
                    push rbx
                    push rax

                    mov rsi, [rsp + 15*8]
                    mov rdi, rsp
                    add rdi, 16*8
                    mov rdx, rsp
                    sub rsp, 8
                    call {}
                    add rsp, 8

                    pop rax
                    pop rbx
                    pop rcx
                    pop rdx
                    pop rbp
                    pop rsi
                    pop rdi
                    pop r8
                    pop r9
                    pop r10
                    pop r11
                    pop r12
                    pop r13
                    pop r14
                    pop r15

                    add rsp, 8
                    iretq
                "#,
                    sym $name,
                    options(noreturn)
                )
            }
        }
        wrapper
    }};
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct IDTDescriptor {
    base_low: u16,
    code_selector: u16,
    ist: u8,
    type_attributes: u8,
    base_mid: u16,
    base_high: u32,
    reserved: u32,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum ISTType {
    None = 0b0000_1110,
    KernelModeIntGate = 0b1000_1110,
    Ring1IntGate = 0b1010_1110,
    Ring2ModeIntGate = 0b1100_1110,
    UserModeIntGate = 0b1110_1110,
}

impl IDTDescriptor {
    pub fn new(ist: u8, typ: ISTType, gdt_selector: u16, handler: extern "C" fn()) -> Self {
        Self {
            code_selector: gdt_selector,
            base_low: handler as u64 as u16,
            base_mid: ((handler as u64) >> 16) as u16,
            base_high: ((handler as u64) >> 32) as u32,
            ist,
            type_attributes: typ as u8,
            reserved: 0,
        }
    }

    pub fn to_u128(self) -> u128 {
        unsafe { *(&self as *const Self as *const _) }
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct Tss {
    _reserved1: u32,
    rsp: [u64; 3],
    _reserved2: u64,
    ist: [u64; 7],
    _reserved3: u64,
    _reserved4: u16,
    iopb_offset: u16,
}

impl Tss {
    pub fn into_slice(self) -> [u64; 13] {
        unsafe { *(&self as *const Self as *const _) }
    }
}

pub struct Interrupts {
    gdt: &'static mut [u64],
    tss: &'static mut [u64],
    idt: &'static mut [u128],
}

impl Interrupts {
    pub fn init(allocator: &mut dyn PhysMem) -> Self {
        let tss_page = allocator
            .alloc_phys_zeroed(Layout::from_size_align(4096, 4096).unwrap())
            .unwrap();

        let tss = unsafe { core::slice::from_raw_parts_mut(tss_page.0 as *mut u64, 13) };

        tss.copy_from_slice(
            &Tss {
                rsp: [
                    &crate::STACK[crate::STACK.len() - 1] as *const u8 as u64,
                    0,
                    0,
                ],
                ..Default::default()
            }
            .into_slice(),
        );

        let gdt_page = allocator
            .alloc_phys_zeroed(Layout::from_size_align(4096, 4096).unwrap())
            .unwrap();

        let gdt = unsafe { core::slice::from_raw_parts_mut(gdt_page.0 as *mut u64, 7) };
        gdt[0] = 0;
        gdt[1] = 0x00209a0000000000; // 0x08 KC
        gdt[2] = 0x0000920000000000; // 0x10 KD
        gdt[3] = 0x0020fb0000000000; // 0x18 UC
        gdt[4] = 0x0000f30000000000; // 0x20 UD

        let tss_base = tss.as_ptr() as u64;
        let tss_low = 0x890000000000
            | (((tss_base >> 24) & 0xff) << 56)
            | ((tss_base & 0xffffff) << 16)
            | (core::mem::size_of::<Tss>() as u64 - 1);
        let tss_high = tss_base >> 32;

        gdt[5] = tss_low;
        gdt[6] = tss_high;

        let mut gdtr = [0u8; 10];
        gdtr[0..2].copy_from_slice(&56u16.to_ne_bytes());
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
            core::arch::asm!(r#"
            mov ax, 0x28
            ltr ax
        "#, lateout("ax") _);
        }

        let idt_page = allocator
            .alloc_phys_zeroed(Layout::from_size_align(4096, 4096).unwrap())
            .unwrap();
        let idt = unsafe { core::slice::from_raw_parts_mut(idt_page.0 as *mut u128, 256) };

        let mut idtr = [0u8; 10];
        idtr[0..2].copy_from_slice(&0xFFFu16.to_ne_bytes());
        idtr[2..10].copy_from_slice(&(idt.as_ptr() as u64).to_ne_bytes());

        unsafe {
            core::arch::asm!("lidt [{}]", in(reg) &idtr);
        }

        idt[0x8] = IDTDescriptor::new(
            0,
            ISTType::KernelModeIntGate,
            0x08,
            handler_errorcode!(double_fault),
        )
        .to_u128();
        idt[0xe] = IDTDescriptor::new(
            0,
            ISTType::KernelModeIntGate,
            0x08,
            handler_errorcode!(page_fault),
        )
        .to_u128();
        idt[0x80] =
            IDTDescriptor::new(0, ISTType::UserModeIntGate, 0x08, handler!(crate::int80)).to_u128();

        Self { gdt, idt, tss }
    }
}

extern "C" fn page_fault(frame: &InterruptFrame, error_code: u64, regs: &Registers) {
    panic!("Page fault {:#x}\n{:#x?}\n{:#x?}", error_code, frame, regs);
}

extern "C" fn double_fault(frame: &InterruptFrame, error_code: u64, regs: &Registers) {
    panic!(
        "Double fault {:#x}\n{:#x?}\n{:#x?}",
        error_code, frame, regs
    );
}
