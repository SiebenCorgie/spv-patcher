use std::{
    fmt::Display,
    io::Write,
    process::{ExitStatus, Stdio},
};
///Takes the SPIR-V words and prints it using spv-dis if installed.
pub struct DisassamblerPrinter {
    to_print: String,
}

impl DisassamblerPrinter {
    pub fn from_bytecode(code: &[u32]) -> Self {
        let mut child = match std::process::Command::new("spirv-dis")
            .arg("--comment")
            .stdin(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to spawn spirv-dis: {}", e);
                return DisassamblerPrinter {
                    to_print: String::from("No spirv-dis"),
                };
            }
        };

        //configure std-in to take our bytecode
        let code: Vec<u8> = bytemuck::cast_slice(code).to_vec();

        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        std::thread::spawn(move || {
            stdin.write_all(&code).expect("Failed to write to stdin");
        });

        let output = match child
            .wait_with_output()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
        {
            Ok(res) => res,
            Err(e) => {
                log::error!("Failed to run spv-dis: {}", e);
                return DisassamblerPrinter {
                    to_print: String::from("Disassambling failed"),
                };
            }
        };
        DisassamblerPrinter { to_print: output }
    }
}

impl Display for DisassamblerPrinter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_print)
    }
}
