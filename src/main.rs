extern crate nix;

use clap::Parser;
use clap::Subcommand;
use nix::fcntl::open;
use nix::fcntl::OFlag;
use nix::sched::{unshare, CloneFlags};
use nix::sys::wait::waitpid;
use nix::unistd::getpid;
use nix::unistd::Pid;
use nix::unistd::{chdir, fork, ForkResult};
use nix::unistd::{close, write};
use std::collections::HashMap;
use std::ffi::{CString, OsString};
use std::fmt::format;
use std::fs::OpenOptions;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::os::fd::RawFd;
use std::os::unix::fs::chroot;
use std::os::unix::thread;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use std::process::{exit, Command};
use std::sync::mpsc::{self, Sender};
use std::time::Duration;
struct LogMessage {
    command: String,
    pid: u32,
    status: String,
}

#[derive(Parser, Debug)]
#[command(
    author = "Nyanpasu",
    version = "1.0",
    about = "minimal conatainerization pet project",
    long_about = "a domain that puts a process in an isolated barrier"
)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Inspects running processes
    Inspect {
        /// Inspect specific PID(s)
        #[clap(short, long)]
        pid: Option<Vec<u32>>,

        /// Inspect all processes
        #[clap(short = 'a', long)]
        all: bool,
    },
    Start {
        // container path
        #[clap(short, long)]
        path: String,

        // env path
        #[clap(short, long)]
        env: String,

        //command
        #[clap(short, long)]
        command: String,
    },
    Tenkai {
        // container path
        #[clap(short, long)]
        path: String,

        // env path
        #[clap(short, long)]
        env: String,

        //command
        #[clap(short, long)]
        command: String,
    },
}

fn start_logger_thread(pid: u32) -> Sender<LogMessage> {
    let (tx, rx) = mpsc::channel::<LogMessage>();
    let log_file_path = format!("./binding_vow/{}.ryouiki", pid);
    let path = PathBuf::from(&log_file_path);
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&path)
        .unwrap_or_else(|e| {
            eprintln!("Failed to open file: {}", e);
            std::process::exit(1);
        });

    let handler = std::thread::Builder::new()
        .name("logger".to_string())
        .spawn(move || {
            let mut log_entries: HashMap<u32, String> = HashMap::new();
            for log_message in rx {
                let entry = format!(
                    "{},{},{}",
                    log_message.command, log_message.pid, log_message.status
                );
                log_entries.insert(log_message.pid, entry);

                for entry in log_entries.values() {
                    writeln!(file, "{}", entry).expect("Failed to write to file");
                }
            }
        });
    handler.expect("Failed to create logger thread");
    tx
}

fn read_file_contents(file_path: &Path) -> io::Result<String> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut content = String::new();
    for line in reader.lines() {
        let line = line?;
        content.push_str(&line);
        content.push('\n');
    }

    Ok(content)
}

fn inspect_specific_processes(pids: &[u32]) {
    for &pid in pids {
        let file_path = PathBuf::from(format!("binding_vow/{}.ryouiki", pid));
        match read_file_contents(&file_path) {
            Ok(content) => {
                println!("--- PID: {} ---\n{}", pid, content);
            }
            Err(e) => {
                eprintln!("Error reading log file for PID {}: {}", pid, e);
            }
        }
    }
}

fn inspect_all_processes() {
    let directory_path = Path::new("binding_vow");

    match fs::read_dir(directory_path) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.is_file() {
                            if let Err(e) = read_file_contents(&path) {
                                eprintln!("Error reading file {:?}: {}", path, e);
                            }
                        }
                    }
                    Err(e) => eprintln!("Error reading directory entry: {}", e),
                }
            }
        }
        Err(e) => eprintln!("Error reading directory {:?}: {}", directory_path, e),
    }
}
//this needs to be automated by getting the Path from the manifest file
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
// this is a direct port from linux unshare.c
fn write_mapping(path: &str, mapping: &str) -> nix::Result<()> {
    let mapping_c = CString::new(mapping).unwrap();

    let fd: RawFd = open(path, OFlag::O_WRONLY, nix::sys::stat::Mode::empty())?;
    write(fd, mapping_c.as_bytes())?;
    close(fd)
}

fn map_root_user() -> nix::Result<()> {
    write_mapping("/proc/self/setgroups", "deny")?;
    write_mapping("/proc/self/uid_map", "0 1000 1")?;
    write_mapping("/proc/self/gid_map", "0 1000 1")
}

fn create_namespaces() {
    match unshare(
        CloneFlags::CLONE_NEWUSER
            | CloneFlags::CLONE_NEWNS
            // | CloneFlags::CLONE_NEWUTS
            | CloneFlags::CLONE_NEWNET,
    ) {
        Ok(_) => (),
        Err(err) => {
            eprintln!("Failed to create a new user namespace: {:?}", err.desc());
            exit(1);
        }
    }
}

fn isolate_filesystem(container_path: &str, os_path: &OsString) {
    chdir(container_path).expect("chdir failed");
    chroot(".").expect("Failed to apply chroot");
    Command::new("sh")
        .arg("-c")
        .arg("mkdir -p /dev/pts")
        .env("PATH", os_path)
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .env("LANGUAGE", "C")
        .spawn()
        .expect("Failed to execute command");
}
fn setup_slirp4netns(child_pid: Pid) {
    // let output = Command::new("slirp4netns")
    //     .args(&[
    //         "--configure",
    //         "--mtu=65520",
    //         "--disable-host-loopback",
    //         &child_pid.to_string(),
    //         "tap0",
    //     ])
    //     .output()
    //     .expect("failed to slirp4netns");
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("echo {} > /tmp/pid", &child_pid))
        .output()
        .expect("failed to register PID");
    let output = Command::new("sh")
        .arg("-c")
        .arg("hacks/slirp4netns.sh")
        .spawn()
        .expect("failed to execute process");
}
fn execute_child_process(command: &str, os_path: &OsString, logger: &Sender<LogMessage>) -> u32 {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .env("PATH", os_path)
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .env("LANGUAGE", "C")
        .spawn()
        .expect("Failed to execute command");

    let pid = child.id();

    // Log the start of the process
    logger
        .send(LogMessage {
            command: command.to_string(),
            pid,
            status: "started".to_string(),
        })
        .expect("Failed to send log message");

    let status = child.wait().expect("Failed to wait on command");

    // Log the completion of the process
    let status_str = if status.success() { "done" } else { "failed" };
    logger
        .send(LogMessage {
            command: command.to_string(),
            pid,
            status: status_str.to_string(),
        })
        .expect("Failed to send log message");

    pid
}

fn tenkai(container_path: &str, env_path: &str, command_to_execute: &str) {
    let env_path = "/home/Nyanpasu/Desktop/code/vscodegit/Ryouiki/assets/manifest/utils/.env";
    let container_path = "/home/Nyanpasu/Desktop/code/vscodegit/Ryouiki/assets/containers/utils";
    fs::create_dir_all("binding_vow").expect("Failed to establish the binding vow"); // Creates the directory if it does not exist
    let path_result = get_path_from_env_file(Path::new(env_path));
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

    let mut sibling = Command::new("sh")
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to start sh process");

    let sibling_pid = sibling.id();
    let pid = getpid();

    create_namespaces();
    // setup_slirp4netns();
    unsafe {
        match fork() {
            Ok(ForkResult::Parent { child, .. }) => {
                // can't run more than one task inside parent
                // match waitpid(child, None) {
                //     Ok(_) => (print!("{}",child) ),
                //     Err(err) => {
                //         eprintln!("waitpid failed: {}", err);
                //         exit(1);
                //     }
                // };
                // setup_slirp4netns(child);
                let command = format!(
                    "slirp4netns  --configure --mtu=65520 --disable-host-loopback {} tap0\n",
                    child
                );
                sibling
                    .stdin
                    .unwrap()
                    .write_all(command.as_bytes())
                    .unwrap();
                match waitpid(child, None) {
                    Ok(_) => (print!("{}", child)),
                    Err(err) => {
                        eprintln!("waitpid failed: {}", err);
                        exit(1);
                    }
                };
            }
            Ok(ForkResult::Child) => {
                match map_root_user() {
                    Ok(_) => {
                        let output = Command::new("sh")
                            .arg("-c")
                            .arg("echo 'nameserver 10.0.2.3' > /tmp/resolv.conf")
                            .output()
                            .expect("failed to execute process");

                        let output = Command::new("sh")
                            .arg("-c")
                            .arg("mount --bind /tmp/resolv.conf /etc/resolv.conf")
                            .output()
                            .expect("failed to execute process");
                        // get the current pid
                        let logger = start_logger_thread(pid.as_raw().try_into().unwrap());
                        // match unshare(
                        //     CloneFlags::CLONE_NEWPID
                        // ) {
                        //     Ok(_) => (),
                        //     Err(err) => {
                        //         eprintln!(
                        //             "Failed to create a new user namespace: {:?}",
                        //             err.desc()
                        //         );
                        //         exit(1);
                        //     }
                        // }
                        // let output = Command::new("sh")
                        //     .arg("-c")
                        //     .arg("unhare --pid --fork")
                        //     .output()
                        //     .expect("failed to execute process");
                        isolate_filesystem(&container_path, &os_path);
                        // std::thread::sleep(Duration::from_secs(2));
                        let command_to_execute: &str = "bash";
                        // let command_to_execute: &str = "ping 8.8.8.8";
                        // "ping 8.8.8.8";
                        let demon_pid =
                            execute_child_process(command_to_execute, &os_path, &logger);
                        println!("Child process, demon pid: {}", demon_pid);
                    }
                    Err(err) => {
                        eprintln!("Mapping root user failed: {}", err.desc());
                        exit(1);
                    }
                }
            }
            Err(err) => {
                eprintln!("fork failed: {}", err);
                exit(1);
            }
        }
    }
    // Continue with child process operations...
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Inspect { pid, all } => {
            if *all {
                inspect_all_processes();
            } else if let Some(pids) = pid {
                inspect_specific_processes(pids);
            } else {
                println!("No specific PIDs provided. Use --all to inspect all processes.");
            }
        }
        Commands::Tenkai { path, env, command } => {
            if (path != "") && (env != "") {
                tenkai(path, env, command);
            }
        }
        Commands::Start { path, env, command } => {
            if (path != "") && (env != "") {
                tenkai(path, env, command);
            } else {
                eprintln!(
                    "Error: Both 'path' and 'env' arguments are required for the start command"
                );
            }
        }
    }
    // tenkai("", "")
}
