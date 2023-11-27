extern crate nix;

use nix::sched::{unshare, CloneFlags};
use nix::sys::wait::waitpid;
use nix::unistd::{chdir, fork, ForkResult};
// use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::chroot;
use std::path::Path;
use std::process::{exit, Command};
use nix::mount::{mount, MsFlags};
use std::time::Duration;
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
fn write_mapping(file: &str, mapping: &str) -> std::io::Result<()> {
    let mut file = File::create(file)?;
    file.write_all(mapping.as_bytes())?;
    Ok(())
}
fn create_namespaces() {
    match unshare(CloneFlags::CLONE_NEWUSER | CloneFlags::CLONE_NEWNS | CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWUTS) {
        Ok(_) => {

            let uid_map = format!("0 {} 1", 1000);
            // let gid_map = format!("0 {} 1", 1000);
        
            // Write the UID and GID mappings
            write_mapping("/proc/self/setgroups", "deny").unwrap();

            // Set UID and GID mappings
            write_mapping("/proc/self/uid_map", "0 1000 1").unwrap(); // Replace 1000 with your UID
            write_mapping("/proc/self/gid_map", "0 1000 1").unwrap(); // Replace 1000 with your GID
        
        
            println!("Successfully created a new user and mount namespace.")
        }
        Err(err) => eprintln!("Failed to create a new user namespace: {:?}", err.desc()),
    }


}

fn isolate_filesystem(os_path: &OsString) {
    chdir("/home/Nyanpasu/Desktop/code/vscodegit/Ryouiki/assets/containers/debian")
        .expect("chdir failed");
    chroot(".").expect("Failed to apply chroot");
    let _ = execute_child_process("mkdir -p /dev/pts", &os_path);
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
    // Command::new("sh").arg("-c").arg("whoami").spawn().unwrap();

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
    create_namespaces();
    // let mut container_info =
    // File::create("container_status.txt").expect("Failed to create file");
    // let mut demon_info = OpenOptions::new()
    //     .write(true)
    //     .append(true)
    //     .create(true)
    //     .open("childern_status.csv")
    //     .expect("Failed to open file");
    unsafe {
        match fork() {
            Ok(ForkResult::Parent { child, .. }) => {
                println!("Parent process, child pid: {}", child);
                Command::new("sh")
                    .arg("-c")
                    .arg("id")
                    .status()
                    .unwrap();
                // let uid_on_host = 1000; // Replace with actual host UID of Nyanpasu
                // let uid_in_ns = 0;
                // let count = 1;

                // let uid_map = format!(" {} {} {} {}", child, uid_in_ns, uid_on_host, count);
                // let gid_map = format!("{} {} {} {}", child, uid_in_ns, uid_on_host, count);
                // println!("uid_map: {}", uid_map);
                // println!("gid_map: {}", gid_map);
                // Command::new("sh")
                //     .arg("-c")
                //     .arg(&format!("newuidmap {}", uid_map))
                //     .status()
                //     .expect("Failed to execute newuidmap");

                // Command::new("sh")
                //     .arg("-c")
                //     .arg(&format!("newgidmap {}", gid_map))
                //     .status()
                //     .expect("Failed to execute newgidmap");

                let _ = nix::sys::wait::waitpid(child, None);
            }
            Ok(ForkResult::Child) => {
                isolate_filesystem(&os_path);
                let command_to_execute: &str =
                    // "while true; do date >> timestamp.log; sleep 10; done";
                "apt-get update";
                // "apt-get update && apt-get install -y apt-utils";
                // "rm -r /dev/null";
                // "ls dev";
                // "whoami";
                let demon_pid = execute_child_process(command_to_execute, &os_path);
                println!("Child process, demon pid: {}", demon_pid);
            }
            Err(e) => eprintln!("First fork failed: {:?}", e),
        }
    }
}
