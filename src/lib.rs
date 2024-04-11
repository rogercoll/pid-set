use std::{collections::HashMap, env::Args};

use libc::{syscall, EPOLL_CTL_DEL};

type FD = i32;
type PID = u32;

// FDPidsMap represents the tracked PIDs and its associated file descriptor
type FDPidsMap = HashMap<PID, FD>;
pub struct PidSet {
    fd_pids: FDPidsMap,
    epoll_fd: FD,
}

impl PidSet {
    pub fn new<P: IntoIterator<Item = PID>>(pids: P) -> Self {
        let epoll_fd = epoll::create(true).unwrap();
        let fd_pids: FDPidsMap = pids
            .into_iter()
            .map(|pid| {
                println!("{pid}");
                // we must provide the flag to avoid collisions
                let cfd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
                // use pid as token
                epoll::ctl(
                    epoll_fd,
                    epoll::ControlOptions::EPOLL_CTL_ADD,
                    cfd as i32,
                    epoll::Event::new(epoll::Events::EPOLLIN, pid as u64),
                )
                .unwrap();
                (pid, cfd as i32)
            })
            .collect();

        Self { fd_pids, epoll_fd }
    }
}

fn wsyscall<F: Fn() -> i32>(syscall: F) -> Result<usize, std::io::Error> {
    let status_code = unsafe { syscall() };
    if status_code < 0 {
        return Err(std::io::Error::from_raw_os_error(status_code));
    }
    Ok(status_code as usize)
}

impl PidSet {
    fn wait(&mut self, n: usize) -> Result<usize, std::io::Error> {
        let max_events = self.fd_pids.len();
        let mut total_events: usize = 0;
        while total_events < n {
            let mut events: Vec<libc::epoll_event> = Vec::with_capacity(max_events);
            total_events += unsafe {
                let event_count =
                    libc::epoll_wait(self.epoll_fd, events.as_mut_ptr(), max_events as i32, -1);
                // TODO: return Error if event_count is -1

                events.set_len(event_count as usize);
                event_count as usize
            };
            println!("Events: {}", events.len());
            events.iter().for_each(|event| {
                // copy needed for alignment
                let cevent = event.events;
                let cdata = event.u64 as u32;
                println!("Deregistering Event: {} {}", cevent, cdata);
                // TODO: return Error if event_count is -1
                let fd = self.fd_pids.get(&cdata).unwrap();
                let status_code = unsafe {
                    libc::epoll_ctl(self.epoll_fd, EPOLL_CTL_DEL, *fd, std::ptr::null_mut())
                };
                println!("Deregister status code: {}", status_code);

                // remove from hashmap
                self.fd_pids.remove(&cdata);
            });

            println!("Total events: {total_events}");
        }
        Ok(total_events)
    }

    pub fn wait_all(&mut self) -> std::io::Result<()> {
        self.wait(self.fd_pids.len());
        Ok(())
    }

    pub fn wait_any(&mut self) {
        self.wait(1);
    }
    pub fn wait_exact<P: IntoIterator<Item = PID>>(pids: P) {
        todo!()
    }
}
