.text
    .global context_switch
    .type context_switch, @function

# void context_switch(uintptr_t *old_rsp_ptr, uintptr_t new_rsp);
# Arguments: rdi = old_rsp_ptr, rsi = new_rsp
context_switch:
    # Save callee-saved registers on current stack
    # Order matches what prepare_stack() sets up
    pushq %r15
    pushq %r14
    pushq %r13
    pushq %r12
    pushq %rbx
    pushq %rbp

    # Store current stack pointer in old task's TCB
    movq %rsp, (%rdi)

    # Switch to new task's stack
    movq %rsi, %rsp

    # Restore callee-saved registers from new stack
    popq %rbp
    popq %rbx
    popq %r12
    popq %r13
    popq %r14
    popq %r15

    # Return to new task (pops entry point address and jumps)
    ret

    .global task_entry_trampoline
    .type task_entry_trampoline, @function

# Trampoline for task entry - ensures proper stack alignment
task_entry_trampoline:
    # Stack should be 16-byte aligned before calling into C code
    # The entry function address is already on top of stack
    # Pop it into a register and call it
    popq %rax
    call *%rax
    
    # If task function returns, terminate the task
    # This should never happen in our cooperative system
    # but provides safety
    movq $0, %rdi  # Exit code 0
    call task_exit
    
    # Infinite loop as safety measure
1:  hlt
    jmp 1b

    .global task_exit
    .type task_exit, @function

# void task_exit(int exit_code)
task_exit:
    # Mark current task as terminated and yield
    # The scheduler will clean up the task
    call scheduler_terminate_current
    # Should never return
1:  hlt
    jmp 1b
