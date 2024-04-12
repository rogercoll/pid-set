//! # PID Set Library
//!
//! `pid_set` is a library for managing and monitoring process identifiers (PIDs) using epoll on Linux.
//! It allows for asynchronous notification when a process exits by leveraging epoll and pidfd (process file descriptors).
//!
//! ## Features
//! - Create a `PidSet` to manage multiple PIDs.
//! - Monitor process exits using epoll.
//! - Handle system call errors gracefully with custom errors.
//!
//! ## Usage
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! pid_set = "0.1.0"
//! ```
//!
//! ## Examples
//! Here's how you can use `PidSet` to monitor a list of PIDs:
//!
//! ```rust
//! use pid_set::{PidSet, PidSetError};
//!
//! fn main() -> Result<(), PidSetError> {
//!     let pids = vec![1234, 5678, 431, 9871, 2123]; // Example PIDs
//!     let mut pid_set = PidSet::new(pids);
//!
//!     // Wait for any PID to exit
//!     pid_set.wait_any()?;
//!
//!     // Clean up
//!     pid_set.close()?;
//!     Ok(())
//! }
//! ```

use std::{collections::HashMap, usize};

use libc::{EPOLLIN, EPOLL_CTL_ADD, EPOLL_CTL_DEL};

type FD = i32;
type PID = u32;

/// A map of process IDs (PIDs) to their associated file descriptors.
type FDPidsMap = HashMap<PID, FD>;

/// Manages a set of PIDs and their corresponding epoll file descriptors.
pub struct PidSet {
    fd_pids: FDPidsMap,
    epoll_fd: Option<FD>,
}

/// Errors that can occur in the `PidSet`.
#[derive(Debug, thiserror::Error)]
pub enum PidSetError {
    #[error("Error while creating epoll file instance:`{0}`")]
    EpollCreate(std::io::Error),

    #[error("Error on pidfd_open syscall for pid `{0}`: `{1}")]
    PidFdOpenSyscall(u32, std::io::Error),

    #[error("Error on epoll_ctl: `{0}")]
    EpollCtl(std::io::Error),

    #[error("Error on epoll_wait: `{0}")]
    EpollWait(std::io::Error),

    #[error("PID not found: `{0}")]
    PidNotFound(u32),

    #[error("Error while closing epoll file descriptor: `{0}")]
    EpollClose(std::io::Error),
}

impl PidSet {
    /// Creates a new `PidSet` with the specified PIDs.
    ///
    /// # Arguments
    ///
    /// * `pids` - An iterator over the PIDs to monitor.
    pub fn new<P: IntoIterator<Item = PID>>(pids: P) -> Self {
        let fd_pids: FDPidsMap = pids.into_iter().map(|pid| (pid, 0)).collect();
        Self {
            fd_pids,
            epoll_fd: None,
        }
    }

    fn register_pid(epoll_fd: i32, pid: u32, token: u32) -> Result<FD, PidSetError> {
        let cfd = unsafe { syscallerr(libc::syscall(libc::SYS_pidfd_open, pid, 0)) }
            .map_err(|err| PidSetError::PidFdOpenSyscall(pid, err))?;
        // use pid as token
        unsafe {
            syserr(libc::epoll_ctl(
                epoll_fd,
                EPOLL_CTL_ADD,
                cfd as i32,
                &mut libc::epoll_event {
                    events: EPOLLIN as u32,
                    u64: token as u64,
                } as *mut _ as *mut libc::epoll_event,
            ))
        }
        .map_err(PidSetError::EpollCtl)?;
        Ok(cfd as i32)
    }

    fn deregister_pid(epoll_fd: i32, fd: i32) -> Result<(), PidSetError> {
        let _ = unsafe {
            syserr(libc::epoll_ctl(
                epoll_fd,
                EPOLL_CTL_DEL,
                fd,
                std::ptr::null_mut(),
            ))
        }
        .map_err(PidSetError::EpollWait)?;
        Ok(())
    }

    fn init_epoll(&mut self) -> Result<FD, PidSetError> {
        // EPOLL_CLOEXEC flag disabled
        let epoll_fd =
            unsafe { syserr(libc::epoll_create1(0)) }.map_err(PidSetError::EpollCreate)?;
        for (pid, fd) in &mut self.fd_pids {
            *fd = PidSet::register_pid(epoll_fd, *pid, *pid)?;
        }

        self.epoll_fd = Some(epoll_fd);
        Ok(epoll_fd)
    }
}

fn syserr(status_code: libc::c_int) -> std::io::Result<libc::c_int> {
    if status_code < 0 {
        return Err(std::io::Error::from_raw_os_error(status_code));
    }
    Ok(status_code)
}

fn syscallerr(status_code: libc::c_long) -> std::io::Result<libc::c_long> {
    if status_code < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(status_code)
}

impl PidSet {
    /// Waits for a specified number of PIDs to exit, up to the total number monitored.
    ///
    /// # Arguments
    ///
    /// * `n` - The number of PID events to wait for.
    ///
    /// # Errors
    ///
    /// Returns `PidSetError` if an error occurs during epoll wait or if a PID is not found.
    fn wait(&mut self, n: usize) -> Result<usize, PidSetError> {
        let max_events = self.fd_pids.len();
        let mut total_events: usize = 0;
        let epoll_fd = self.epoll_fd.unwrap_or(self.init_epoll()?);
        while total_events < n {
            let mut events: Vec<libc::epoll_event> = Vec::with_capacity(max_events);
            let event_count = syserr(unsafe {
                libc::epoll_wait(epoll_fd, events.as_mut_ptr(), max_events as i32, -1)
            })
            .map_err(PidSetError::EpollWait)? as usize;
            unsafe { events.set_len(event_count as usize) };
            total_events += event_count;

            for event in events {
                let cdata = event.u64 as u32;
                // TODO: return Error if event_count is -1
                let fd = self
                    .fd_pids
                    .get(&cdata)
                    .ok_or(PidSetError::PidNotFound(cdata))?;
                PidSet::deregister_pid(epoll_fd, *fd)?;

                // remove from hashmap
                self.fd_pids.remove(&cdata);
            }
        }
        Ok(total_events)
    }

    /// Waits for all PIDs to exit.
    ///
    /// # Errors
    ///
    /// Returns `PidSetError` if an error occurs during the wait.
    pub fn wait_all(&mut self) -> Result<(), PidSetError> {
        self.wait(self.fd_pids.len())?;
        Ok(())
    }

    /// Waits for any one PID to exit.
    ///
    /// # Errors
    ///
    /// Returns `PidSetError` if an error occurs during the wait.
    pub fn wait_any(&mut self) -> Result<(), PidSetError> {
        self.wait(1)?;
        Ok(())
    }

    /// Closes the epoll file descriptor and cleans up the `PidSet`.
    ///
    /// # Errors
    ///
    /// Returns `PidSetError` if an error occurs while closing the epoll file descriptor.
    pub fn close(mut self) -> Result<(), PidSetError> {
        let epoll_fd = self.epoll_fd.unwrap_or(self.init_epoll()?);
        unsafe { syserr(libc::close(epoll_fd)) }.map_err(PidSetError::EpollClose)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn sleep_cmd(duration: &str) -> std::process::Command {
        let mut cmd1 = std::process::Command::new("sleep");
        cmd1.arg(duration);
        cmd1
    }

    #[test]
    fn wait_all() {
        let mut pid_set = PidSet::new([
            sleep_cmd("0.1").spawn().unwrap().id(),
            sleep_cmd("0.2").spawn().unwrap().id(),
            sleep_cmd("0.3").spawn().unwrap().id(),
            sleep_cmd("0.4").spawn().unwrap().id(),
            sleep_cmd("0.5").spawn().unwrap().id(),
        ]);

        assert!(pid_set.wait_all().is_ok());
    }

    #[test]
    fn wait_any() {
        let start_time = Instant::now(); // Start the timer

        let mut pid_set = PidSet::new([
            sleep_cmd("0.2").spawn().unwrap().id(),
            sleep_cmd("3").spawn().unwrap().id(),
            sleep_cmd("3").spawn().unwrap().id(),
            sleep_cmd("3").spawn().unwrap().id(),
            sleep_cmd("3").spawn().unwrap().id(),
        ]);

        assert!(pid_set.wait_any().is_ok());
        assert!(
            start_time.elapsed() < Duration::from_secs(3),
            "Expected wait_any() to return in less than 3 seconds, but it took {:?}",
            start_time.elapsed()
        );
    }
}
