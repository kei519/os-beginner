use core::{
    arch::{asm, global_asm},
    ffi::c_char,
};

use crate::task::TaskContext;

pub fn halt() -> ! {
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

pub fn io_out_32(addr: u16, data: u32) {
    unsafe { io_out_32_unsafe(addr, data) }
}

pub fn io_in_32(addr: u16) -> u32 {
    unsafe { io_in_32_unsafe(addr) }
}

pub fn get_cs() -> u16 {
    unsafe { get_cs_unsafe() }
}

pub fn load_idt(limit: u16, offset: u64) {
    unsafe { load_idt_unsafe(limit, offset) }
}

pub fn load_gdt(limit: u16, offset: u64) {
    unsafe { load_gdt_unsafe(limit, offset) }
}

pub fn set_ds_all(value: u16) {
    unsafe { set_ds_all_unsafe(value) }
}

pub fn set_cs_ss(cs: u16, ss: u16) {
    unsafe { set_cs_ss_unsafe(cs, ss) }
}

pub fn set_cr3(value: u64) {
    unsafe { set_cr3_unsafe(value) }
}

pub fn get_cr3() -> u64 {
    unsafe { get_cr3_unsafe() }
}

pub fn switch_context(next_ctx: &TaskContext, current_ctx: &TaskContext) {
    unsafe { switch_context_unsafe(next_ctx, current_ctx) }
}

pub fn sti() {
    unsafe { asm!("sti") }
}

pub fn sti_hlt() {
    unsafe { asm!("sti", "hlt") }
}

pub fn cli() {
    unsafe { asm!("cli") }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_app(argc: i32, argv: *const *const c_char, cs: u16, ss: u16, rip: u64, rsp: u64) {
    unsafe { call_app_unsafe(argc, argv, cs, ss, rip, rsp) };
}

extern "C" {
    fn io_out_32_unsafe(addr: u16, data: u32);
    fn io_in_32_unsafe(addr: u16) -> u32;
    fn get_cs_unsafe() -> u16;
    fn load_idt_unsafe(limit: u16, offset: u64);
    fn load_gdt_unsafe(limit: u16, offset: u64);
    fn set_ds_all_unsafe(value: u16);
    fn set_cs_ss_unsafe(cs: u16, ss: u16);
    fn set_cr3_unsafe(value: u64);
    fn get_cr3_unsafe() -> u64;
    fn switch_context_unsafe(next_ctx: &TaskContext, current_ctx: &TaskContext);
    fn call_app_unsafe(argc: i32, argv: *const *const c_char, cs: u16, ss: u16, rip: u64, rsp: u64);
}

global_asm! { r#"
.global io_out_32_unsafe
io_out_32_unsafe:
    mov dx, di
    mov eax, esi
    out dx, eax
    ret

.global io_in_32_unsafe
io_in_32_unsafe:
    mov dx, di
    in eax, dx
    ret

.global get_cs_unsafe
get_cs_unsafe:
    xor eax, eax
    mov ax, cs
    ret

.global load_idt_unsafe
load_idt_unsafe:
    push rbp
    mov rbp, rsp
    sub rsp, 10
    mov [rsp], di
    mov [rsp + 2], rsi
    lidt [rsp]
    mov rsp, rbp
    pop rbp
    ret

.global load_gdt_unsafe
load_gdt_unsafe:
    push rbp
    mov rbp, rsp
    sub rsp, 10
    mov [rsp], di # limit
    mov [rsp + 2], rsi # offset
    lgdt [rsp]
    mov rsp, rbp
    pop rbp
    ret

.global set_ds_all_unsafe
set_ds_all_unsafe:
    mov ds, di
    mov es, di
    mov fs, di
    mov gs, di
    ret

.global set_cs_ss_unsafe
set_cs_ss_unsafe:
    push rbp
    mov rbp, rsp
    mov ss, si
    lea rax, .next
    push rdi    # CS
    push rax    # RIP
    retfq
.next:
    mov rsp, rbp
    pop rbp
    ret

.global set_cr3_unsafe
set_cr3_unsafe:
    mov cr3, rdi
    ret

.global get_cr3_unsafe
get_cr3_unsafe:
    mov rax, cr3
    ret

.global switch_context_unsafe
switch_context_unsafe: # switch_context_unsafe(next_ctx, current_ctx)
    mov [rsi + 0x40], rax
    mov [rsi + 0x48], rbx
    mov [rsi + 0x50], rcx
    mov [rsi + 0x58], rdx
    mov [rsi + 0x60], rdi
    mov [rsi + 0x68], rsi

    lea rax, [rsp + 8]
    mov [rsi + 0x70], rax # RSP
    mov [rsi + 0x78], rbp

    mov [rsi + 0x80], r8
    mov [rsi + 0x88], r9
    mov [rsi + 0x90], r10
    mov [rsi + 0x98], r11
    mov [rsi + 0xa0], r12
    mov [rsi + 0xa8], r13
    mov [rsi + 0xb0], r14
    mov [rsi + 0xb8], r15

    mov rax, cr3
    mov [rsi + 0x00], rax # CR3
    mov rax, [rsp]
    mov [rsi + 0x08], rax # RIP
    pushfq
    pop qword ptr [rsi + 0x10] # RFLAGS

    mov ax, cs
    mov [rsi + 0x20], RAX
    mov bx, ss
    mov [rsi + 0x28], RBX
    mov cx, fs
    mov [rsi + 0x30], RCX
    mov dx, gs
    mov [rsi + 0x38], RDX

    fxsave [rsi + 0xc0]

    # iret 用のスタックフレーム
    push qword ptr [rdi + 0x28] # SS
    push qword ptr [rdi + 0x70] # RSP
    push qword ptr [rdi + 0x10] # RFLAGS
    push qword ptr [rdi + 0x20] # CS
    push qword ptr [rdi + 0x08] # RIP

    # コンテキストの復帰
    fxrstor [rdi + 0xc0]

    mov rax, [rdi + 0x00]
    mov cr3, rax
    mov rax, [rdi + 0x30]
    mov fs, ax
    mov rax, [rdi + 0x38]
    mov gs, ax

    mov rax, [rdi + 0x40]
    mov rbx, [rdi + 0x48]
    mov rcx, [rdi + 0x50]
    mov rdx, [rdi + 0x58]
    mov rsi, [rdi + 0x68]
    mov rbp, [rdi + 0x78]
    mov r8, [rdi + 0x80]
    mov r9, [rdi + 0x88]
    mov r10, [rdi + 0x90]
    mov r11, [rdi + 0x98]
    mov r12, [rdi + 0xa0]
    mov r13, [rdi + 0xa8]
    mov r14, [rdi + 0xb0]
    mov r15, [rdi + 0xb8]

    mov rdi, [rdi + 0x60]

    iretq

.global call_app_unsafe
call_app_unsafe:
    push rbp
    mov rbp, rsp
    push rcx # SS
    push r9 # RSP
    push rdx # CS
    push r8 # RIP
    retfq
    # アプリケーションが ret してもここには来ない
"# }
