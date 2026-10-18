use std::process::Command;
use pealayer::four_d::process_supervisor::ManagedChild;

#[test]
fn test_managed_child_spawns_and_terminates() {
    #[cfg(unix)]
    let mut cmd = Command::new("sleep");
    #[cfg(unix)]
    cmd.arg("60");

    #[cfg(windows)]
    let mut cmd = Command::new("ping");
    #[cfg(windows)]
    cmd.args(["127.0.0.1", "-n", "60"]);

    let managed = ManagedChild::spawn(cmd).expect("spawn managed child");
    assert!(managed.id() > 0);
    drop(managed);
}
