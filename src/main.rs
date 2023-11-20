extern crate nix;

use nix::sched::{unshare, CloneFlags};
use nix::unistd::{chdir, fork, ForkResult};
// use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, ErrorKind, Read, Result, Write};
use std::os::unix::fs::chroot;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

// #[derive(Serialize, Deserialize, Debug)]
// struct Config {
//     Env: Vec<String>,
// }

// fn load_manifest() -> Result<Value> {
//     let file_content = fs::read_to_string("./assets/manifest/debian/manifest.json")?;
//     serde_json::from_str(&file_content)
//         .map_err(|err| io::Error::new(ErrorKind::Other, format!("JSON parsing error: {}", err)))
// }

// fn get_path_from_manifest(json: &Value) -> Option<String> {
//     json.as_array()
//         .and_then(|arr| arr.first())
//         .and_then(|first_manifest| {
//             first_manifest
//                 .get("Config")
//                 .and_then(|c| c.get("Env"))
//                 .and_then(|env| env.as_array())
//                 .and_then(|env_array| env_array.first())
//                 .and_then(|p| p.as_str())
//                 .map(|s| s.to_string())
//         })
// }

fn get_path_from_env_file(file_path: &Path) -> io::Result<Option<String>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    for line_result in reader.lines() {
        let line = line_result?;
        if line.starts_with("PATH=") {
            return Ok(line.strip_prefix("PATH=").map(|s| s.to_string()));
        }
    }

    Ok(None)
}
fn create_namespace() {
    match unshare(CloneFlags::CLONE_NEWUSER | CloneFlags::CLONE_NEWNS) {
        // match unshare(CloneFlags::CLONE_NEWUSER | CloneFlags::CLONE_NEWNS | CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWNET) {
        Ok(_) => println!("Successfully created a new user and mount namespace."),
        Err(err) => eprintln!("Failed to create a new user namespace: {:?}", err),
    }
}

fn isolate_filesystem() {
    chdir("/home/Nyanpasu/Desktop/code/vscodegit/Ryouiki/assets/containers/debian")
        .expect("chdir failed");
    chroot(".").expect("Failed to apply chroot");
}

fn execute_child_process(command: &str, os_path: &OsString) -> u32 {
    // Command::new("sh")
    //     .arg("-c")
    //     .arg("locale-gen en_US.UTF-8")
    //     .env("PATH", os_path)
    //     .spawn()
    //     .expect("Failed to execute command");
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .env("PATH", os_path)
        // .env("LANG", "en_US.UTF-8")
        // .env("LC_ALL", "en_US.UTF-8")
        // .env("LANGUAGE", "en_US:en")
        // .env("TERM", "xterm-256color")
        .spawn()
        .expect("Failed to execute command");
    child.wait().expect("Failed to wait on command");

    let pid = child.id();

    pid
}

fn main() {
    create_namespace();

    // Attempt to load the manifest and extract the path
    // let path_result = load_manifest().and_then(|json| {
    //     get_path_from_manifest(&json).ok_or_else(|| {
    //         std::io::Error::new(std::io::ErrorKind::NotFound, "Path not found in manifest")
    //     })
    // });
    let path_result = get_path_from_env_file(Path::new(
        "/home/Nyanpasu/Desktop/code/vscodegit/Ryouiki/assets/manifest/debian/.env",
    ));

    // Check the result and handle errors
    let os_path = match path_result {
        Ok(Some(path)) => OsString::from(path),
        Ok(None) => {
            eprintln!("Error: {}", "Path not found in manifest");
            return; // Or handle the error as appropriate for your application
        }
        Err(err) => {
            eprintln!("Error: {}", err);
            return; // Or handle the error as appropriate for your application
        }
    };

    // fn prepare_environment() {
    //     // Run these commands in the isolated environment before executing your main command
    //     let _ = execute_command("mount -t devpts devpts /dev/pts");
    //     let _ = Command::new("sh")
    //     .arg("-c")
    //     .arg(command)
    //     .env("PATH", os_path)
    //     .spawn()
    //     .expect("Failed to execute command");
    //     // If necessary, install apt-utils or any other required packages
    // }

    unsafe {
        let mut container_info =
            File::create("container_status.txt").expect("Failed to create file");
        let mut demon_info = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open("childern_status.csv")
            .expect("Failed to open file");
        match fork() {
            Ok(ForkResult::Parent { child, .. }) => {
                println!("Parent process, child pid: {}", child);
                let _ = nix::sys::wait::waitpid(child, None);
                writeln!(container_info, "Container with PID {} is running", child)
                    .expect("Failed to write to file");
            }
            Ok(ForkResult::Child) => {
                match fork() {
                    Ok(ForkResult::Parent { .. }) => {
                        std::process::exit(0); // First child exits
                    }
                    Ok(ForkResult::Child) => {
                        match fork() {
                            Ok(ForkResult::Parent { child, .. }) => {
                                println!("demon pid: {}", child);
                                std::process::exit(0)
                            } // First child exits
                            Ok(ForkResult::Child) => {
                                nix::unistd::setsid().expect("Failed to create new session");
                                isolate_filesystem();
                                // let command_to_execute =
                                //     "while true; do date >> timestamp.log; sleep 10; done";
                                let command_to_execute: &str = "apt-get -y install locales";
                                let demon_pid = execute_child_process(command_to_execute, &os_path);

                                writeln!(
                                    demon_info,
                                    "{},{}",
                                    &command_to_execute,
                                    &demon_pid.to_string()
                                )
                                .expect("Failed to write to file");
                                demon_info.flush().expect("Failed to flush file");
                            }
                            Err(_) => println!("Second fork failed"),
                        }
                    }
                    Err(_) => println!("Fork failed"),
                }
            }
            Err(_) => println!("Fork failed"),
        }
    }
}
