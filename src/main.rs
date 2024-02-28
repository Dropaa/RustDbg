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
use crate::working::set_breakpoint;

fn run_command(command: &str, child: unistd::Pid) {
    let args: Vec<&str> = command.split_whitespace().collect();
    match args.get(0) {
        Some(&"c" | &"continue") => {
            println!("Continuing execution...");
            if let Err(err) = ptrace::cont(child, None) {
                println!("Failed to continue execution: {:?}", err);
            } else {
                prettier(child);
            }
        }
        Some(&"s" | &"syscall") => {
            if let Err(err) = waitpid(child, None) {
                println!("Failed to wait: {:?}", err);
                return;
            }
            let registers_syscall = match ptrace::getregs(child) {
                Ok(registers) => registers,
                Err(err) => {
                    println!("Could not get child's registers: {:?}", err);
                    return;
                }
            };
            let _syscall_name = syscall::syscall_name(registers_syscall.orig_rax);
            println!("Entering {} ({}) syscall", _syscall_name, registers_syscall.orig_rax);
            if let Err(err) = ptrace::syscall(child, None) {
                println!("Failed to use PTRACE_SYSCALL: {:?}", err);
            }
        }
        Some(&"n" | &"next") => {
            println!("Taking a single step...");
            if let Err(err) = ptrace::step(child, None) {
                println!("Failed to continue execution: {:?}", err);
            }
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
        Some(&"b" | &"breakpoint") => {
            if args.len() != 2 {
                println!("Usage: b <address>");
                return;
            }
            let hex_address = args[1];
            if !hex_address.starts_with("0x") {
                println!("Your address should start with 0x !");
                return;
            }
            let hex_address = &hex_address[2..];
            match u64::from_str_radix(hex_address, 16) {
                Ok(address) => {
                    if let Err(err) = set_breakpoint(child, address) {
                        println!("Failed to set breakpoint: {:?}", err);
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
