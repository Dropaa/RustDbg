use nix::sys::ptrace;
use nix::unistd;
use std::collections::HashMap;

static mut BREAKPOINTS: Option<HashMap<u64, u8>> = None;



/// Set a breakpoint at the specified memory address in the debugged process.
///
/// # Arguments
///
/// * `child` - The process ID (Pid) of the child being debugged.
/// * `address` - The memory address where the breakpoint is to be set.
///
/// # Errors
///
/// Returns an error if setting the breakpoint fails.
///
/// # Safety
///
/// This function involves modifying the debugged process's memory and relies on unsafe operations.
///
pub fn set_breakpoint(child: unistd::Pid, address: u64) -> Result<(), nix::Error> {
    let original_byte = ptrace::read(child, address as nix::sys::ptrace::AddressType)?;

    unsafe {
        if let Some(ref mut breakpoints) = BREAKPOINTS {
            breakpoints.insert(address, original_byte as u8);
        } else {
            let mut breakpoints = HashMap::<u64, u8>::new();
            breakpoints.insert(address, original_byte as u8);
            BREAKPOINTS = Some(breakpoints);
        }
    }

    let word_to_write = (original_byte & !0xff) | 0xcc;
    unsafe { ptrace::write(child, address as nix::sys::ptrace::AddressType, word_to_write as nix::sys::ptrace::AddressType) }?;

    Ok(())
}


/// Handle a breakpoint hit at the specified address in the debugged process.
///
/// # Arguments
///
/// * `child` - The process ID (Pid) of the child being debugged.
/// * `address` - The memory address where the breakpoint was hit.
///
pub fn handle_breakpoint(child: unistd::Pid, address: u64) {
    unsafe {
        if let Some(ref mut breakpoints) = BREAKPOINTS {
            if let Some(&original_byte) = breakpoints.get(&address) {
                let mut original_instruction = ptrace::read(child, address as nix::sys::ptrace::AddressType)
                    .expect("Failed to read original instruction");
                // Restaurer l'instruction d'origine à l'adresse du breakpoint
                // En remplaçant uniquement le dernier octet par l'octet original
                original_instruction &= !0xff;
                original_instruction |= original_byte as i64;

                // Écrire l'instruction restaurée dans la mémoire du processus enfant
                ptrace::write(child, address as nix::sys::ptrace::AddressType, original_instruction as nix::sys::ptrace::AddressType)
                    .expect("Failed to restore original instruction");

                println!("Hit breakpoint at address {:#x}", address);
                return;
            }
        }
    }
    println!("Hit unknown breakpoint at address {:#x}", address);
}


/// Handle process stopping events and print information when a SIGTRAP signal is received.
///
/// This function continuously waits for the child process to stop and checks if it's due to a SIGTRAP signal,
/// indicating a breakpoint hit. When a SIGTRAP is detected, it prints information about it and then breaks
/// out of the loop.
///
/// # Arguments
///
/// * `child` - The process ID (Pid) of the child being debugged.
///
/// # Panics
///
/// This function panics if it fails to get the register states of the child process.

pub fn prettier(child: unistd::Pid) {
    loop {
        match nix::sys::wait::waitpid(child, None) {
            Ok(status) => {
                if status == nix::sys::wait::WaitStatus::Stopped(child, nix::sys::signal::Signal::SIGTRAP) {
                    println!("SIGTRAP");
                    let regs = ptrace::getregs(child).expect("Failed to get registers");
                    let rip = regs.rip as u64;
                    handle_breakpoint(child, rip - 1);
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

/// Print register states of the debugged process.
///
/// # Arguments
///
/// * `child` - The process ID (Pid) of the child being debugged.
///
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
    println!("  rip: 0x{:x}", regs.rip);
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

/// Print available debugger commands and their descriptions.
pub fn help_commands() {
    println!("Available commands:");
    println!("  c or continue: Continue the process until completion (or the next breakpoint)");
    println!("  s or syscall: Continue the process until the next syscall (or end of syscall)");
    println!("  n or next: Make a single step in the process (Continue to next instruction (single-step))");
    println!("  r or registers: Show the register states of the process");
    println!("  m or memory: Show the content of a memory address");
    println!("  h or help: Enter an instruction to get the list of available instructions.");
}