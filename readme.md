# Ryouiki

<div style="text-align:center;">
  <img src="him/nahidwin.png" alt="Stand Proud">
</div>


## Overview

Ryouiki implements a minimal rootless containerization system. It's designed to isolate processes in a Linux environment, providing a lightweight alternative to full-scale container engines. The system creates isolated environments (containers) where processes can run independently from the host and each other.

## Features

- **Process Isolation**: Utilizes Linux namespaces and cgroups for process isolation.
- **Customizable Environments**: Allows specifying custom paths for container and environment setup.
- **Logging and Inspection**: Provides functionalities to log container activities and inspect running processes.
- **Network Namespacing**: Implements basic network isolation and configuration.

## Prerequisites

- Rust and Cargo (latest stable version).
- Linux environment with support for namespaces.
- slirp4netns

## Installation

1. Clone the repository:

   ```sh
   git clone [repository-url]
   ```

2. Navigate to the project directory:

   ```sh
   cd [project-directory]
   ```

3. Build the project:
   ```sh
   cargo build --release
   ```

## Usage

This implementation is rootless, meaning it does not require root privileges to create and manage containers. This approach enhances security and accessibility.

Before using the commands, ensure you have prepared the necessary environment and filesystem for the container.

### Preparing the Environment

1. **Environment File**: Create a text file containing the `PATH` environment variable. This file should specify the path to the binaries and libraries inside your container.

   Example of an environment file (`env.txt`):

   ```
   PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
   ```

2. **Exporting the Filesystem**: Prepare the filesystem for the container. This involves setting up a directory structure that will act as the root filesystem for your containerized processes. Ensure that all necessary binaries, libraries, and dependencies are available within this filesystem.

   Example structure:

   ```
   /path/to/container
   ├── bin
   ├── lib
   ├── usr
   └── ...
   ```

### Basic Commands

After preparing the environment and filesystem, you can use the following commands:

- **Starting a process inside the containerized environment (Tenkai)**: To execute a custom command in a containerized environment. Provide the paths to the container's filesystem and the environment file.
  ```sh
  ryouiki tenkai --path /path/to/container --env /path/to/env.txt --command [command-to-execute]
  ```

### Note

- Ensure the paths provided for the container's filesystem and the environment file are correct and accessible.
- The command specified in the `--command` argument is executed inside the containerized environment.

By following these steps, users can successfully create and manage containerized environments using your minimal containerization project.

### Basic Commands

- **Inspecting Processes**: To inspect specific or all running processes within the containerized environment.
  ```sh
  cargo run -- inspect [--pid PID1,PID2,...] [--all]
  ```
- **Starting a Container**: To start a new containerized process.
  ```sh
  cargo run -- start --path [container-path] --env [env-path] --command [command-to-execute]
  ```
- **Custom Command (Tenkai)**: To execute a custom command in a containerized environment.
  ```sh
  cargo run -- tenkai --path [container-path] --env [env-path] --command [command-to-execute]
  ```

### Log Management

Logs for each containerized process are stored in the `binding_vow` directory with the naming convention `[PID].ryouiki`.

## Disclaimer

This is a minimal implementation intended for educational purposes. It may not be suitable for production use.


## credits

- [missingsemester](https://missingsemester.io/)
- [podman](https://github.com/containers/podman)
- chatgpt
- random strangers on the internet❤️
