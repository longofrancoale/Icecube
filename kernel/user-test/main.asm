section .text
global _start
_start:
    xor rax, rax
    int 0x80
    xor rbx, rbx
    dec rbx
    cmp rax, rbx
    je .error
    jmp _start
.error:
    jmp $