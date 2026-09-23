bits 64
global isr_page_fault
global isr_syscall
global enter_user_mode
global kernel_after_user
extern page_fault_handler
extern syscall_handler
extern rust_kernel_after_user
extern process_exit_dispatch
extern rust_ring3_done

; Syscall frame layout after pushes:
; 0=RAX,1=RBX,2=RCX,3=RDX,4=RSI,5=RDI,6=RBP,
; 7=R8,8=R9,9=R10,10=R11,11=R12,12=R13,13=R14,14=R15.
isr_page_fault:
    push rax
    push rdi
    push rsi
    mov rsi, [rsp+24]
    mov rdi, cr2
    call page_fault_handler
    pop rsi
    pop rdi
    pop rax
    add rsp, 8
    iretq

isr_syscall:
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    mov al, 0x53
    mov dx, 0x3F8
    out dx, al
    mov rdi, rsp
    call syscall_handler

    cmp rax, 0xDEAD
    je .do_exit

    ; syscall_handler returns the value for userspace in RAX.
    ; RAX is frame slot 0, so restore it from there after all pops.
    mov [rsp], rax
    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax
    iretq

.do_exit:
    add rsp, 15*8
    add rsp, 5*8
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    call process_exit_dispatch
    jmp rust_ring3_done

enter_user_mode:
    cli
    mov ax, 0x23
    mov ds, ax
    mov es, ax
    push 0x23
    push rsi
    push 0x2
    push 0x1B
    push rdi
    iretq

kernel_after_user:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    call rust_kernel_after_user
.hang:
    hlt
    jmp .hang
