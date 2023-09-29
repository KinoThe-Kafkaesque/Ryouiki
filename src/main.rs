extern crate nix;

use nix::sched::{unshare, CloneFlags};
use nix::unistd::{chdir, execv, fork, ForkResult};
use std::ffi::CString;
use std::process::{Command, Stdio};

fn main() {
    // Step 1: Create a new user namespace
    match unshare(CloneFlags::CLONE_NEWUSER | CloneFlags::CLONE_NEWNS) {
        Ok(_) => {
            println!("Successfully created a new user and mount namespace.");
            
            // Here, you'd typically set up ID mappings from the host to the new user namespace
            
            // Execute some code to test (e.g., running a shell)
            let output = Command::new("whoami")
                .output()
                .expect("Failed to execute command");
                
            let output_str = String::from_utf8_lossy(&output.stdout);
            println!("Executed command, got output: {}", output_str);
        },
        Err(err) => {
            eprintln!("Failed to create a new user namespace: {:?}", err);
        }
    }

    unsafe {
        match fork() {
            Ok(ForkResult::Parent { child, .. }) => {
                println!(
                    "Continuing execution in parent process, new child has pid: {}",
                    child
                );
            }
            Ok(ForkResult::Child) => {
                // Step 2: Isolate the filesystem using the mount namespace
                chdir("/home/Nyanpasu/Desktop/code/vscodegit/Ryouiki/assets").expect("chdir failed");

                // Step 3: Run /bin/bash
                Command::new("bash")
                .arg("-i") // run bash in interactive mode
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("Failed to start bash")
                .wait()
                .expect("Failed to wait on bash");
            }
            Err(_) => {
                println!("Fork failed");
            }
        }
    }
}
