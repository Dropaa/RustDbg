//! # Rust Debugger
//!
//! This debugger is designed to trace and debug programs written in Rust. It provides basic debugging functionalities
//! such as stepping through code, examining register states, viewing memory contents, setting breakpoints, and more.
//!
//! ## Usage
//!
//! To use the debugger, simply run it with the path to the program you want to debug as a command-line argument:
//!
//! ```sh
//! cargo run <program_path>
//! ```
//!
//! Once the debugger is running, you'll be prompted with a debug console (rustdbg>). You can input various commands
//! to control the debugger's behavior.
//!
//! ## Commands
//!
//! The following commands are supported:
//!
//! - `c` or `continue`: Continue program execution.
//! - `s` or `syscall`: Step into the next system call.
//! - `n` or `next`: Execute the next line of code.
//! - `r` or `registers`: Display register states.
//! - `m <address>` or `memory <address>`: View the memory contents at a specified address.
//! - `b <address>` or `breakpoint <address>`: Set a breakpoint at a specified address.
//! - `h` or `help`: Display help information.
//! - `q` or `quit`: Exit the debugger.
//!
//! ## Example
//!
//! ```sh
//! $ cargo run /path/to/program
//! Child pid: 1234
//! rustdbg> c
//! Continuing execution...
//! rustdbg>
//! ```
//!
//! ## Testing
//!
//! Unit tests are provided to ensure the correctness of debugger functionalities. Run the tests using:
//!
//! ```sh
//! cargo test
//! ```
//!
//! ## Modules
//!
//! - `syscall`: Provides utilities to work with system calls.
//! - `working`: Contains various functions for debugger operations.
//!
//! ## Note
//!
//! This debugger relies on the `nix` crate for system-level operations and process management.
//!
//! ## Safety
//!
//! This debugger involves low-level operations and interacts with system resources. Care must be taken while using it
//! to avoid unintended consequences or system instability.
//!
//! ## Authors
//!
//! - Ryan HENNOU
//! - Aurelien KRIEF
//! 
//! 
//! 
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


/// Executes the specified command in the debugger.
///
/// # Arguments
///
/// * `command` - A string slice representing the command to execute.
/// * `child` - The process ID (Pid) of the child being debugged.
///
/// # Example
///
/// ```rust
/// run_command("c", child_pid);
/// ```
///

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

/// Entry point of the debugger application.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: cargo run <program_path>");
        return;
    }
    let program_path = &args[1];
    let path: &CStr = &CString::new(program_path.clone()).unwrap();

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

#[cfg(test)]
mod test;
