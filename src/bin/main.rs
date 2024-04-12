fn sleep_cmd(duration: &str) -> std::process::Command {
    let mut cmd1 = std::process::Command::new("sleep");
    cmd1.arg(duration);
    cmd1
}

fn main() {
    let mut pid_set = pid_set::PidSet::new([
        sleep_cmd("1").spawn().unwrap().id(),
        sleep_cmd("3").spawn().unwrap().id(),
        sleep_cmd("3").spawn().unwrap().id(),
        sleep_cmd("3").spawn().unwrap().id(),
        sleep_cmd("3").spawn().unwrap().id(),
    ])
    .unwrap();
    pid_set.wait_all().unwrap();

    pid_set.close().unwrap()
}
