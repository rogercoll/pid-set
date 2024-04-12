[![Crates.io][crates-badge]][crates-url]
[![CI](https://github.com/rogercoll/pid-set/actions/workflows/test.yaml//badge.svg?branch=main)](https://github.com/rogercoll/pid-set/actions/workflows/test.yaml?query=branch%3Amain)
[![Dependency status](https://deps.rs/repo/github/rogercoll/pid-set/status.svg)](https://deps.rs/repo/github/rogercoll/pid-set)


[crates-badge]: https://img.shields.io/crates/v/pid-set.svg
[crates-url]: https://crates.io/crates/pid-set

# PID Set Library

The `pid_set` library provides tools for managing and monitoring process identifiers (PIDs) using epoll on Linux platforms. It utilizes epoll and pidfd to asynchronously notify when a process exits, offering a robust way to handle PID-related events.

## Motivation

The primary motivation behind developing the PidSet crate stems from a limitation in Rust's standard library, particularly with the [std::process::Child struct](https://doc.rust-lang.org/std/process/struct.Child.html), which is used to represent child processes. The standard Child struct provides a [wait](https://doc.rust-lang.org/std/process/struct.Child.html#method.wait) and a [try_wait](https://doc.rust-lang.org/std/process/struct.Child.html#method.try_wait) method that blocks the entire system thread until the child process exits. This blocking behavior is not ideal for efficiently managing multiple child processes within a single thread.

The PidSet crate addresses this by leveraging Linux's epoll and pidfd functionalities, enabling non-blocking and asynchronous monitoring of multiple processes. By using epoll, PidSet allows a program to "watch" multiple child processes and receive notifications about changes in their state (like termination), without the need of blocking a thread for each child process. This is particularly useful in applications that need to manage several child processes concurrently without dedicating a separate thread to each process just to wait for its completion.

### Man pages
 - https://man7.org/linux/man-pages/man2/pidfd_open.2.html
 - https://man7.org/linux/man-pages/man2/epoll_create.2.html
 - https://man7.org/linux/man-pages/man2/epoll_ctl.2.html

## Features

- **Manage Multiple PIDs**: Track and manage multiple process identifiers easily.
- **Asynchronous Monitoring**: Use epoll for efficient event notification.
- **Error Handling**: Includes comprehensive error handling to manage system call failures gracefully.

### WIP features

- [ ] Insert/delete function
- [ ] Add duration parameter to wait functions (breaking change)

## Prerequisites

This library is intended for use on Linux systems with support for `epoll` and `pidfd_open`. Ensure your system meets these requirements before using the library.


## Installation

Add `pid_set` to your `Cargo.toml` dependencies:

```toml
[dependencies]
pid_set = "0.1.0"
```

## Usage

```rust
use pid_set::{PidSet, PidSetError};

fn main() -> Result<(), PidSetError> {
    // Example PIDs to monitor
    let pids = vec![1234, 5678];
    let mut pid_set = PidSet::new(pids);

    // Wait for any one PID to exit
    pid_set.wait_any()?;

    // Clean up
    pid_set.close()?;
    Ok(())
}
```

## API

 - `PidSet::new(pids)`: Create a new PidSet with the specified PIDs.
 - `PidSet::wait_any()`: Wait for any one PID to exit.
 - `PidSet::wait_all()`: Wait for all PIDs to exit.
 - `PidSet::close()`: Close the epoll file descriptor and clean up resources.
