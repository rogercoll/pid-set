fn sleep_cmd(duration: &str) -> std::process::Command {
    let mut cmd1 = std::process::Command::new("sleep");
    cmd1.arg(duration);
    cmd1
}

fn main() {
    cmd_supervisor::PidSet::new([
        sleep_cmd("1").spawn().unwrap().id(),
        sleep_cmd("3").spawn().unwrap().id(),
        sleep_cmd("3").spawn().unwrap().id(),
        sleep_cmd("3").spawn().unwrap().id(),
        sleep_cmd("3").spawn().unwrap().id(),
    ])
    .wait_all()
    .unwrap()
}
