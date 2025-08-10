.text

# Common interrupt handler that saves all registers
.macro SAVE_REGISTERS
    pushq %rax
    pushq %rbx
    pushq %rcx
    pushq %rdx
    pushq %rsi
    pushq %rdi
    pushq %rbp
    pushq %r8
    pushq %r9
    pushq %r10
    pushq %r11
    pushq %r12
    pushq %r13
    pushq %r14
    pushq %r15
.endm

# Restore all registers from stack
.macro RESTORE_REGISTERS
    popq %r15
    popq %r14
    popq %r13
    popq %r12
    popq %r11
    popq %r10
    popq %r9
    popq %r8
    popq %rbp
    popq %rdi
    popq %rsi
    popq %rdx
    popq %rcx
    popq %rbx
    popq %rax
.endm

# Timer interrupt handler (IRQ 0)
    .global timer_interrupt_stub
    .type timer_interrupt_stub, @function
timer_interrupt_stub:
    SAVE_REGISTERS
    
    # Call Rust timer handler
    call timer_interrupt_handler_rust
    
    # Send EOI to PIC
    movb $0x20, %al
    outb %al, $0x20
    
    RESTORE_REGISTERS
    iretq

# Keyboard interrupt handler (IRQ 1)
    .global keyboard_interrupt_stub
    .type keyboard_interrupt_stub, @function
keyboard_interrupt_stub:
    SAVE_REGISTERS
    
    # Read scancode from keyboard port
    inb $0x60, %al
    movzbl %al, %edi  # Zero-extend to 32-bit for function call
    
    # Call Rust keyboard handler
    call keyboard_interrupt_handler_rust
    
    # Send EOI to PIC
    movb $0x20, %al
    outb %al, $0x20
    
    RESTORE_REGISTERS
    iretq

# RTC interrupt handler (IRQ 8)
    .global rtc_interrupt_stub
    .type rtc_interrupt_stub, @function
rtc_interrupt_stub:
    SAVE_REGISTERS
    
    # Call Rust RTC handler
    call rtc_interrupt_handler_rust
    
    # Send EOI to both PICs (RTC is on slave PIC)
    movb $0x20, %al
    outb %al, $0xA0  # Slave PIC
    outb %al, $0x20  # Master PIC
    
    RESTORE_REGISTERS
    iretq

# System call interrupt handler (INT 0x80)
    .global syscall_interrupt_stub
    .type syscall_interrupt_stub, @function
syscall_interrupt_stub:
    SAVE_REGISTERS
    
    # Syscall number in %rax, args in %rdi, %rsi, %rdx, %rcx, %r8, %r9
    # Call Rust syscall dispatcher
    movq %rax, %rdi  # Syscall number as first argument
    call syscall_handler_rust
    
    RESTORE_REGISTERS
    iretq

# Exception handlers

# Division by zero (INT 0)
    .global divide_error_stub
    .type divide_error_stub, @function
divide_error_stub:
    SAVE_REGISTERS
    call divide_error_handler_rust
    RESTORE_REGISTERS
    iretq

# Debug exception (INT 1)
    .global debug_stub
    .type debug_stub, @function
debug_stub:
    SAVE_REGISTERS
    call debug_handler_rust
    RESTORE_REGISTERS
    iretq

# Breakpoint (INT 3)
    .global breakpoint_stub
    .type breakpoint_stub, @function
breakpoint_stub:
    SAVE_REGISTERS
    call breakpoint_handler_rust
    RESTORE_REGISTERS
    iretq

# Invalid opcode (INT 6)
    .global invalid_opcode_stub
    .type invalid_opcode_stub, @function
invalid_opcode_stub:
    SAVE_REGISTERS
    call invalid_opcode_handler_rust
    RESTORE_REGISTERS
    iretq

# General protection fault (INT 13) - has error code
    .global general_protection_fault_stub
    .type general_protection_fault_stub, @function
general_protection_fault_stub:
    # Error code is already on stack
    SAVE_REGISTERS
    
    # Pass error code as argument (it's on stack above saved registers)
    movq 120(%rsp), %rdi  # Error code location (15 regs * 8 bytes)
    call general_protection_fault_handler_rust
    
    RESTORE_REGISTERS
    addq $8, %rsp  # Remove error code from stack
    iretq

# Page fault (INT 14) - has error code
    .global page_fault_stub
    .type page_fault_stub, @function
page_fault_stub:
    # Error code is already on stack
    SAVE_REGISTERS
    
    # Get faulting address from CR2
    movq %cr2, %rsi
    
    # Pass error code and faulting address as arguments
    movq 120(%rsp), %rdi  # Error code
    call page_fault_handler_rust
    
    RESTORE_REGISTERS
    addq $8, %rsp  # Remove error code from stack
    iretq

# Double fault (INT 8) - has error code, never returns
    .global double_fault_stub
    .type double_fault_stub, @function
double_fault_stub:
    # Error code is on stack but we don't care - this is fatal
    SAVE_REGISTERS
    
    movq 120(%rsp), %rdi  # Error code
    call double_fault_handler_rust
    
    # Should never return
1:  hlt
    jmp 1b

# Spurious interrupt handler
    .global spurious_interrupt_stub
    .type spurious_interrupt_stub, @function
spurious_interrupt_stub:
    # Don't send EOI for spurious interrupts
    iretq

# Default interrupt handler for unhandled interrupts
    .global default_interrupt_stub
    .type default_interrupt_stub, @function
default_interrupt_stub:
    SAVE_REGISTERS
    
    # Get interrupt number from stack frame
    # This is a simplification - in practice you'd need to track this differently
    movq $0xFF, %rdi  # Unknown interrupt marker
    call default_interrupt_handler_rust
    
    # Send EOI anyway to be safe
    movb $0x20, %al
    outb %al, $0x20
    
    RESTORE_REGISTERS
    iretq

# Helper macros for generating interrupt stubs
.macro INTERRUPT_STUB_NO_ERROR name, vector
    .global \name
    .type \name, @function
\name:
    pushq $0        # Push dummy error code
    pushq $\vector  # Push interrupt vector
    jmp common_interrupt_handler
.endm

.macro INTERRUPT_STUB_ERROR name, vector
    .global \name
    .type \name, @function
\name:
    pushq $\vector  # Push interrupt vector (error code already on stack)
    jmp common_interrupt_handler
.endm

# Common interrupt handler that processes vector number
common_interrupt_handler:
    SAVE_REGISTERS
    
    # Get vector number and error code from stack
    movq 120(%rsp), %rdi  # Vector number
    movq 128(%rsp), %rsi  # Error code
    
    call common_interrupt_handler_rust
    
    RESTORE_REGISTERS
    addq $16, %rsp  # Remove vector and error code
    iretq

# Generate stubs for all 256 possible interrupts
# This allows dynamic interrupt handler registration

# Exceptions (0-31)
INTERRUPT_STUB_NO_ERROR interrupt_0, 0    # Divide Error
INTERRUPT_STUB_NO_ERROR interrupt_1, 1    # Debug
INTERRUPT_STUB_NO_ERROR interrupt_2, 2    # NMI
INTERRUPT_STUB_NO_ERROR interrupt_3, 3    # Breakpoint
INTERRUPT_STUB_NO_ERROR interrupt_4, 4    # Overflow
INTERRUPT_STUB_NO_ERROR interrupt_5, 5    # Bound Range Exceeded
INTERRUPT_STUB_NO_ERROR interrupt_6, 6    # Invalid Opcode
INTERRUPT_STUB_NO_ERROR interrupt_7, 7    # Device Not Available
INTERRUPT_STUB_ERROR    interrupt_8, 8    # Double Fault
INTERRUPT_STUB_NO_ERROR interrupt_9, 9    # Coprocessor Segment Overrun
INTERRUPT_STUB_ERROR    interrupt_10, 10  # Invalid TSS
INTERRUPT_STUB_ERROR    interrupt_11, 11  # Segment Not Present
INTERRUPT_STUB_ERROR    interrupt_12, 12  # Stack-Segment Fault
INTERRUPT_STUB_ERROR    interrupt_13, 13  # General Protection Fault
INTERRUPT_STUB_ERROR    interrupt_14, 14  # Page Fault
INTERRUPT_STUB_NO_ERROR interrupt_15, 15  # Reserved
INTERRUPT_STUB_NO_ERROR interrupt_16, 16  # x87 FPU Error
INTERRUPT_STUB_ERROR    interrupt_17, 17  # Alignment Check
INTERRUPT_STUB_NO_ERROR interrupt_18, 18  # Machine Check
INTERRUPT_STUB_NO_ERROR interrupt_19, 19  # SIMD Floating-Point Exception
INTERRUPT_STUB_NO_ERROR interrupt_20, 20  # Virtualization Exception
INTERRUPT_STUB_ERROR    interrupt_21, 21  # Control Protection Exception

# Reserved exceptions (22-31)
INTERRUPT_STUB_NO_ERROR interrupt_22, 22
INTERRUPT_STUB_NO_ERROR interrupt_23, 23
INTERRUPT_STUB_NO_ERROR interrupt_24, 24
INTERRUPT_STUB_NO_ERROR interrupt_25, 25
INTERRUPT_STUB_NO_ERROR interrupt_26, 26
INTERRUPT_STUB_NO_ERROR interrupt_27, 27
INTERRUPT_STUB_NO_ERROR interrupt_28, 28
INTERRUPT_STUB_NO_ERROR interrupt_29, 29
INTERRUPT_STUB_NO_ERROR interrupt_30, 30
INTERRUPT_STUB_NO_ERROR interrupt_31, 31

# Hardware interrupts (32-47) - PIC remapped
INTERRUPT_STUB_NO_ERROR interrupt_32, 32  # Timer
INTERRUPT_STUB_NO_ERROR interrupt_33, 33  # Keyboard
INTERRUPT_STUB_NO_ERROR interrupt_34, 34  # Cascade
INTERRUPT_STUB_NO_ERROR interrupt_35, 35  # COM2
INTERRUPT_STUB_NO_ERROR interrupt_36, 36  # COM1
INTERRUPT_STUB_NO_ERROR interrupt_37, 37  # LPT2
INTERRUPT_STUB_NO_ERROR interrupt_38, 38  # Floppy
INTERRUPT_STUB_NO_ERROR interrupt_39, 39  # LPT1
INTERRUPT_STUB_NO_ERROR interrupt_40, 40  # RTC
INTERRUPT_STUB_NO_ERROR interrupt_41, 41  # Free
INTERRUPT_STUB_NO_ERROR interrupt_42, 42  # Free
INTERRUPT_STUB_NO_ERROR interrupt_43, 43  # Free
INTERRUPT_STUB_NO_ERROR interrupt_44, 44  # PS/2 Mouse
INTERRUPT_STUB_NO_ERROR interrupt_45, 45  # FPU
INTERRUPT_STUB_NO_ERROR interrupt_46, 46  # Primary ATA
INTERRUPT_STUB_NO_ERROR interrupt_47, 47  # Secondary ATA

# User-defined interrupts (48-255)
# These can be used for system calls or other purposes
INTERRUPT_STUB_NO_ERROR interrupt_128, 128  # Common syscall vector

# Create a jump table for easy lookup
.section .rodata
    .global interrupt_handler_table
    .type interrupt_handler_table, @object
interrupt_handler_table:
    .quad interrupt_0, interrupt_1, interrupt_2, interrupt_3
    .quad interrupt_4, interrupt_5, interrupt_6, interrupt_7
    .quad interrupt_8, interrupt_9, interrupt_10, interrupt_11
    .quad interrupt_12, interrupt_13, interrupt_14, interrupt_15
    .quad interrupt_16, interrupt_17, interrupt_18, interrupt_19
    .quad interrupt_20, interrupt_21, interrupt_22, interrupt_23
    .quad interrupt_24, interrupt_25, interrupt_26, interrupt_27
    .quad interrupt_28, interrupt_29, interrupt_30, interrupt_31
    .quad interrupt_32, interrupt_33, interrupt_34, interrupt_35
    .quad interrupt_36, interrupt_37, interrupt_38, interrupt_39
    .quad interrupt_40, interrupt_41, interrupt_42, interrupt_43
    .quad interrupt_44, interrupt_45, interrupt_46, interrupt_47
    # Add more entries as needed for higher interrupt numbers
