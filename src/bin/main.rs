fn sleep_cmd(duration: &str) -> std::process::Command {
    let mut cmd1 = std::process::Command::new("sleep");
    cmd1.arg(duration);
    cmd1
}

fn main() {
    cmd_supervisor::CommandSupervisor::spawn([
        sleep_cmd("30"),
        sleep_cmd("30"),
        sleep_cmd("30"),
        sleep_cmd("30"),
        sleep_cmd("30"),
    ])
    .wait_one()
}
