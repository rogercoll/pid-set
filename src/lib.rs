type FD = i64;

// FDChild represents a child and its associated file descriptor
type FDChild = (FD, std::process::Child);

pub struct UnSpawned;
pub struct Spawned {
    fd_childs: Vec<FDChild>,
}

pub struct CommandSupervisor<S = UnSpawned> {
    state: S,
}

impl CommandSupervisor<UnSpawned> {
    pub fn spawn<I: IntoIterator<Item = std::process::Command>>(
        commands: I,
    ) -> CommandSupervisor<Spawned> {
        let fd_childs: Vec<FDChild> = commands
            .into_iter()
            .map(|mut command| {
                let child = command.spawn().unwrap();
                let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, child.id()) };
                (fd, child)
            })
            .collect();

        CommandSupervisor {
            state: Spawned { fd_childs },
        }
    }
}

impl CommandSupervisor<Spawned> {
    pub fn wait_one(&mut self) {
        // TODO: change to libc
        let epfd = epoll::create(false).unwrap();
        self.state
            .fd_childs
            .iter()
            .enumerate()
            .for_each(|(i, (cfd, _))| {
                // TODO: think about which token to use (i)
                epoll::ctl(
                    epfd,
                    epoll::ControlOptions::EPOLL_CTL_ADD,
                    *cfd as i32,
                    epoll::Event::new(epoll::Events::EPOLLIN, i as u64),
                )
                .unwrap()
            });
        let max_events = self.state.fd_childs.len();
        let mut events: Vec<libc::epoll_event> = Vec::with_capacity(max_events);
        let output = unsafe { libc::epoll_wait(epfd, events.as_mut_ptr(), max_events as i32, -1) };
        // let output = epoll::wait(epfd, -1, events.as_mut_slice()).unwrap();
        println!("{output}");
        // println!("{:?}", events);
    }
}
