extern crate nix;

use nix::libc::{c_char, mount};
use nix::sched::{unshare, CloneFlags};
use nix::unistd::{chdir, fork, ForkResult};
// use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, ErrorKind, Read, Result, Write};
use std::os::unix::fs::chroot;
use std::path::Path;
use std::process::Command;
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

fn create_namespaces() {
    match unshare(CloneFlags::CLONE_NEWUSER | CloneFlags::CLONE_NEWNS) {
        Ok(_) => {
            // Get the current user ID in the parent namespace
            let uid_output = Command::new("id").arg("-u").output();
            match uid_output {
                Ok(output) => {
                    if !output.status.success() {
                        eprintln!("Failed to get user ID");
                    } else {
                        let uid = std::str::from_utf8(&output.stdout).unwrap_or("").trim();
                        println!("Current User ID: {}", &uid);
                    }
                }
                Err(err) => {
                    eprintln!("Failed to run command: {}", err);
                }
            }
            println!("Successfully created a new user and mount namespace.")
        }
        Err(err) => eprintln!("Failed to create a new user namespace: {:?}", err),
    }
}
fn mount_dev_pts() -> std::result::Result<(), std::io::Error> {
    let mkdir_output = Command::new("sh")
        .arg("-c")
        .arg("mkdir -p /dev/pts")
        .output()?;

    if !mkdir_output.status.success() {
        let err_msg = std::str::from_utf8(&mkdir_output.stderr)
            .unwrap_or("Failed to read error message")
            .trim();
        eprintln!("Failed to create /dev/pts directory: {}", err_msg);
        return Err(io::Error::new(io::ErrorKind::Other, "Mkdir command failed"));
    }

    // Attempt to mount /dev/pts
    let mount_output = Command::new("sh")
        .arg("-c")
        .arg("mount -t devpts devpts /dev/pts")
        .output()?;

    if !mount_output.status.success() {
        let err_msg = std::str::from_utf8(&mount_output.stderr)
            .unwrap_or("Failed to read error message")
            .trim();
        eprintln!("Failed to mount /dev/pts: {}", err_msg);
        return Err(io::Error::new(io::ErrorKind::Other, "Mount command failed"));
    }

    Ok(())
}
fn isolate_filesystem() {
    chdir("/home/Nyanpasu/Desktop/code/vscodegit/Ryouiki/assets/containers/debian")
        .expect("chdir failed");
    chroot(".").expect("Failed to apply chroot");
    mount_dev_pts().expect("Failed to mount devpts");
}

fn execute_child_process(command: &str, os_path: &OsString) -> u32 {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .env("PATH", os_path)
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .env("LANGUAGE", "C")
        // .env("TERM", "xterm-256color")
        .spawn()
        .expect("Failed to execute command");
    child.wait().expect("Failed to wait on command");

    let pid = child.id();

    pid
}

fn main() {
    create_namespaces();
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
                                let command_to_execute: &str =
                                    "apt-get update && apt-get install -y apt-utils";
                                // "ls /dev";
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
