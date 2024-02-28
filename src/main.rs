use std::ffi::{CStr, CString};
use std::io::{self, Write};
use nix::sys::ptrace;
use nix::unistd::{self, fork, ForkResult};
use nix::sys::wait::waitpid;
mod syscall;
mod working;
use crate::working::prettier;
use crate::working::show_registers;
use crate::working::help_commands;

fn run_command(command: &str, child: unistd::Pid) {
    let args: Vec<&str> = command.split_whitespace().collect();
    match args.get(0) {
        Some(&"c" | &"continue") => {
            println!("Continuing execution...");
            ptrace::cont(child, None).expect("Failed to continue execution");
            prettier(child);
        }
        Some(&"s" | &"syscall") => {
            let _ = waitpid(child, None).expect("Failed to wait");
            let registers_syscall = ptrace::getregs(child).expect("Could not get child's registers");
            let _syscall_name = syscall::syscall_name(registers_syscall.orig_rax);
            println!("Entering {} ({}) syscall", _syscall_name, registers_syscall.orig_rax);
            ptrace::syscall(child, None).expect("Failed to use PTRACE_SYSCALL");
        }
        Some(&"n" | &"next") => {
            println!("Taking a single step...");
            ptrace::step(child, None).expect("Failed to continue execution");
        }
        Some(&"r" | &"registers") => {
            println!("Showing register states...");
            show_registers(child);
        }
        Some(&"m" | &"memory") => {
            if args.len() != 2 {
                println!("Usage: m <address>");
                return;
            }
            let hex_address = args[1];
            if !hex_address.starts_with("0x") {
                println!("Your address should start with 0x !");
                return;
            }
            let hex_address = &hex_address[2..]; // Removing "0x" prefix
            match u64::from_str_radix(hex_address, 16) {
                Ok(address) => {
                    match ptrace::read(child, address as nix::sys::ptrace::AddressType) {
                        Ok(value) => println!("{:#018x}", value),
                        Err(_) => println!("Not able to read the content of this address"),
                    }
                }
                Err(_) => println!("Invalid address format"),
            }
        }
        Some(&"h" | &"help") => {
            help_commands();
        }
        Some(&"q" | &"quit") => {
            println!("Exiting the debugger !");
            std::process::exit(0);
        }
        _ => println!("Unknown command: {}", command),
    }
}

fn main() {
    let path: &CStr = &CString::new("/home/dropa/sample_db_rust/target/debug/sample_db_rust").unwrap();
    match unsafe { fork() }.expect("Failed to fork") {
        ForkResult::Parent { child } => {
            println!("Child pid: {}", child);
            loop {
                print!("rustdbg> ");
                io::stdout().flush().expect("Failed to flush stdout");
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read line");

                let input = input.trim().trim_end_matches(&['\r', '\n'][..]);

                run_command(input, child);
            }
        }
        ForkResult::Child => {
            ptrace::traceme().expect("Failed to call traceme in child");
            nix::unistd::execve::<&CStr, &CStr>(path, &[], &[]).unwrap();
        }
    }
}
