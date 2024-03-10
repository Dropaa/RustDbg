#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::{Read, Write}, process::{Command, Stdio}};

    #[test]
    fn test_command_execution() {
        let output = execute_debugger_command("r").expect("Failed to execute debugger command");
        assert!(output.contains("Registers:"), "Debugger failed to display register states");
    }

    #[test]
    fn test_register_display() {
        let output = execute_debugger_command("r").expect("Failed to execute debugger command");
        assert!(output.contains("rax:") && output.contains("rsp:"), "Debugger failed to display all registers");
    }

    #[test]
    fn test_memory_display() {
        let output = execute_debugger_command("m 0x12345678").expect("Failed to execute debugger command");
        assert!(output.contains("Memory content at address 0x12345678:"), "Debugger failed to display memory content");
    }

    #[test]
    fn test_help_command() {
        let output = execute_debugger_command("h").expect("Failed to execute debugger command");
        assert!(output.contains("Available commands:"), "Debugger failed to display help information");
    }

    #[test]
    fn test_invalid_commands() {
        let output = execute_debugger_command("invalid").expect("Failed to execute debugger command");
        assert!(output.contains("Unknown command"), "Debugger failed to handle unknown commands");
    }

    fn execute_debugger_command(command: &str) -> Result<String, std::io::Error> {
        let mut debugger_process = Command::new("target/debug/dbg_rust")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        
        debugger_process.stdin.as_mut().unwrap().write_all(command.as_bytes())?;
        
        let mut output = String::new();
        debugger_process.stdout.as_mut().unwrap().read_to_string(&mut output)?;
        
        debugger_process.wait()?;
        
        Ok(output)
    }
}