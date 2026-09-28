bits 64
global isr_page_fault
global isr_syscall
global isr_wifi_irq
global enter_user_mode
global kernel_after_user
extern page_fault_handler
extern syscall_handler
extern wifi_irq_handler
extern rust_kernel_after_user
extern process_exit_dispatch
extern rust_ring3_done

; Syscall frame layout after pushes (stack top is index 0):
; 0=R15,1=R14,2=R13,3=R12,4=R11,5=R10,6=R9,
; 7=R8,8=RBP,9=RDI,10=RSI,11=RDX,12=RCX,13=RBX,14=RAX.
; The iret frame starts at index 15: RIP,CS,RFLAGS,RSP,SS.
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
    ; RAX is frame slot 14, so restore it there before all pops.
    mov [rsp+14*8], rax
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


isr_wifi_irq:
    push rax
    push rcx
    push rdx
    push rsi
    push rdi
    call wifi_irq_handler
    mov al, 0x20
    mov dx, 0x20
    out dx, al
    mov dx, 0xA0
    out dx, al
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rax
    iretq
