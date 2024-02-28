
use nix::sys::ptrace;
use nix::unistd;

pub fn prettier(child: unistd::Pid) {
    loop {
        match nix::sys::wait::waitpid(child, None) {
            Ok(status) => {
                if status == nix::sys::wait::WaitStatus::Stopped(child, nix::sys::signal::Signal::SIGTRAP) {
                    break;
                }
            }
            Err(e) if nix::errno::Errno::from_raw(e as i32) == nix::errno::Errno::ECHILD => {
                // The child process has already terminated.
                println!("rustdbg> Child process has terminated.");
                std::process::exit(0);
            }
            Err(e) => {
                // Handle other errors if needed
                eprintln!("Error: {}", e);
                break;
            }
        }
    }
}

pub fn show_registers(child: unistd::Pid) {
    let regs = ptrace::getregs(child).expect("Failed to get registers");
    println!("Registers:");
    println!("  rax: 0x{:x}", regs.rax);
    println!("  rbx: 0x{:x}", regs.rbx);
    println!("  rcx: 0x{:x}", regs.rcx);
    println!("  rdx: 0x{:x}", regs.rdx);
    println!("  rsi: 0x{:x}", regs.rsi);
    println!("  rdi: 0x{:x}", regs.rdi);
    println!("  rsp: 0x{:x}", regs.rsp);
    println!("  rbp: 0x{:x}", regs.rbp);
    println!("  r8 : 0x{:x}", regs.r8);
    println!("  r9 : 0x{:x}", regs.r9);
    println!("  r10: 0x{:x}", regs.r10);
    println!("  r11: 0x{:x}", regs.r11);
    println!("  r12: 0x{:x}", regs.r12);
    println!("  r13: 0x{:x}", regs.r13);
    println!("  r14: 0x{:x}", regs.r14);
    println!("  r15: 0x{:x}", regs.r15);
}

pub fn help_commands() {
    println!("Available commands:");
    println!("  c or continue: Continue the process until completion (or the next breakpoint)");
    println!("  s or syscall: Continue the process until the next syscall (or end of syscall)");
    println!("  n or next: Make a single step in the process (Continue to next instruction (single-step))");
    println!("  r or registers: Show the register states of the process");
    println!("  m or memory: Show the content of a memory address");
    println!("  h or help: Enter an instruction to get the list of available instructions.");
}