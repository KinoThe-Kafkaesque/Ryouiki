extern crate nix;

use nix::sched::{unshare, CloneFlags};
use nix::unistd::{chdir, fork, ForkResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::Result;
use std::os::unix::fs::chroot;
use std::process::{Command, Stdio};
use std::fs::File;
use std::io::Write;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    Env: Vec<String>,
}

fn load_manifest() -> Result<Value> {
    // Read the JSON file
    let file_content = fs::read_to_string("./assets/manifest/ubuntu/manifest.json")?;

    // Parse the JSON file
    let json: Value = serde_json::from_str(&file_content)?;

    Ok(json)
}

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

    //Step 2: load the manifest and set env vars and PATH

    let path = match load_manifest() {
        Ok(json) => {
            // Assuming json is an array and we're interested in the first element
            if let Some(first_manifest) = json.as_array().and_then(|arr| arr.first()) {
                // Access Config and then Env
                if let Some(env_array) = first_manifest.get("Config").and_then(|c| c.get("Env").and_then(|env| env.as_array())) {
                    // Extract PATH, assuming it's the first element
                    env_array.first().and_then(|p| p.as_str()).map(|s| s.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        }
        Err(e) => {
            eprintln!("Error loading manifest: {:?}", e);
            None
        }
    };
    let path_str = path.clone().unwrap_or_default();
    let path_os_str = std::ffi::OsString::from(path_str);
    let path_ref: &std::ffi::OsStr = path_os_str.as_ref();
    print!("PATH: {:?}\n", &path_ref.to_str().unwrap().split('=').nth(1).expect("Invalid PATH format"));

// Wait for the bash process to finish

    unsafe {
        match fork() {
            Ok(ForkResult::Parent { child, .. }) => {
                println!(
                    "Continuing execution in parent process, new child has pid: {}",
                    child
                );
                let _ = nix::sys::wait::waitpid(child, None);
                let mut file = File::create("container_status.txt").expect("Failed to create file");
                writeln!(file, "Container with PID {} is running", child).expect("Failed to write to file");
            }
            Ok(ForkResult::Child) => {
                // Step 3: Isolate the filesystem using the mount namespace
                chdir("/home/Nyanpasu/Desktop/code/vscodegit/Ryouiki/assets/containers/ubuntu").expect("chdir failed");
                chroot(".").expect("Failed to apply chroot");

                // set the PATH
                // std::env::set_var("PATH", &path.unwrap());
                
                // Step 4: Run /bin/bash
                Command::new("bash")
                // .env_clear()
                .env("HOME", "/home")
                .env("PATH", &path_ref.to_str().unwrap().split('=').nth(1).expect("Invalid PATH format")) // This exports the PATH to the bash process
                // .stdin(Stdio::inherit())
                // .stdout(Stdio::inherit())
                // .stderr(Stdio::inherit())
                .spawn()
                .expect("Failed to start bash");
                // .wait()
                // .expect("Failed to wait on bash");
            }
            Err(_) => {
                println!("Fork failed");
            }
        }
    }

}
