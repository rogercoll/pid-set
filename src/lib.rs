use std::{collections::HashMap, usize};

use libc::{EPOLLIN, EPOLL_CTL_ADD, EPOLL_CTL_DEL};

type FD = i32;
type PID = u32;

// FDPidsMap represents the tracked PIDs and its associated file descriptor
type FDPidsMap = HashMap<PID, FD>;
pub struct PidSet {
    fd_pids: FDPidsMap,
    epoll_fd: FD,
}

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
    pub fn new<P: IntoIterator<Item = PID>>(pids: P) -> Result<Self, PidSetError> {
        // EPOLL_CLOEXEC flag disabled
        let epoll_fd =
            unsafe { syserr(libc::epoll_create1(0)) }.map_err(PidSetError::EpollCreate)?;
        let fd_pids: Result<FDPidsMap, PidSetError> = pids
            .into_iter()
            .map(|pid| {
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
                            u64: pid as u64,
                        } as *mut _ as *mut libc::epoll_event,
                    ))
                }
                .map_err(PidSetError::EpollCtl)?;
                Ok((pid, cfd as i32))
            })
            .collect();

        Ok(Self {
            fd_pids: fd_pids?,
            epoll_fd,
        })
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
    fn wait(&mut self, n: usize) -> Result<usize, PidSetError> {
        let max_events = self.fd_pids.len();
        let mut total_events: usize = 0;
        while total_events < n {
            let mut events: Vec<libc::epoll_event> = Vec::with_capacity(max_events);
            let event_count = syserr(unsafe {
                libc::epoll_wait(self.epoll_fd, events.as_mut_ptr(), max_events as i32, -1)
            })
            .map_err(PidSetError::EpollWait)? as usize;
            unsafe { events.set_len(event_count as usize) };
            total_events += event_count;
            println!("Events: {}", events.len());

            for event in events {
                let cevent = event.events;
                let cdata = event.u64 as u32;
                println!("Deregistering Event: {} {}", cevent, cdata);
                // TODO: return Error if event_count is -1
                let fd = self
                    .fd_pids
                    .get(&cdata)
                    .ok_or(PidSetError::PidNotFound(cdata))?;
                let status_code = unsafe {
                    syserr(libc::epoll_ctl(
                        self.epoll_fd,
                        EPOLL_CTL_DEL,
                        *fd,
                        std::ptr::null_mut(),
                    ))
                }
                .map_err(PidSetError::EpollWait)?;
                println!("Deregister status code: {}", status_code);

                // remove from hashmap
                self.fd_pids.remove(&cdata);
            }

            println!("Total events: {total_events}");
        }
        Ok(total_events)
    }

    pub fn wait_all(&mut self) -> Result<(), PidSetError> {
        self.wait(self.fd_pids.len())?;
        Ok(())
    }

    pub fn wait_any(&mut self) -> Result<(), PidSetError> {
        self.wait(1)?;
        Ok(())
    }

    pub fn close(self) -> Result<(), PidSetError> {
        unsafe { syserr(libc::close(self.epoll_fd)) }.map_err(PidSetError::EpollClose)?;
        Ok(())
    }
}
