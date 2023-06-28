use std::io::Write;
use std::process::{Command, Stdio};
///`spirv-val` based validator.
//TODO: Might add custom validation passes later.
#[allow(dead_code)]
pub struct Validator;

#[allow(dead_code)]
impl Validator {
    ///Tries to run validator. Returns `Ok` if validated successfully
    /// or `Err` containing `spirv-val`'s error if not.
    pub fn validate_code(spirv: &[u8]) -> Result<(), String> {
        let mut child = match Command::new("spirv-val").stdin(Stdio::piped()).spawn() {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to spawn spirv-val: {}", e);
                return Err(String::from(
                    "Failed to find spirv-val, is it installed and in $PATH?",
                ));
            }
        };

        //configure std-in to take our bytecode
        let code = spirv.to_vec();

        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        std::thread::spawn(move || {
            stdin.write_all(&code).expect("Failed to write to stdin");
        });

        match child
            .wait_with_output()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
        {
            Ok(_res) => Ok(()),
            Err(e) => {
                log::error!("Failed to run spv-dis: {}", e);
                Err(format!("{}", e))
            }
        }
    }
}
