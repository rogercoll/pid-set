# PID Set Library

The `pid_set` library provides tools for managing and monitoring process identifiers (PIDs) using epoll on Linux platforms. It utilizes epoll and pidfd to asynchronously notify when a process exits, offering a robust way to handle PID-related events.

## Features

- **Manage Multiple PIDs**: Track and manage multiple process identifiers easily.
- **Asynchronous Monitoring**: Use epoll for efficient event notification.
- **Error Handling**: Includes comprehensive error handling to manage system call failures gracefully.

## Prerequisites

This library is intended for use on Linux systems with support for `epoll` and `pidfd_open`. Ensure your system meets these requirements before using the library.

### Man pages
 - https://man7.org/linux/man-pages/man2/pidfd_open.2.html
 - https://man7.org/linux/man-pages/man2/epoll_create.2.html
 - https://man7.org/linux/man-pages/man2/epoll_ctl.2.html

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
    let mut pid_set = PidSet::new(pids)?;

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
