//! Signal forwarding: SIGINT/SIGTERM sent to the CLI are forwarded
//! to the running child, and the child's exit code is propagated.
//!
//! This lives in its own integration-test binary: the forwarding handler and
//! the raised signal are process-wide, so the test must not share a process
//! with other `SystemRunner` tests.

use tug::process::{Invocation, Runner, SystemRunner};

unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
    fn getpid() -> i32;
}

const SIGTERM: i32 = 15;

#[test]
fn system_runner_forwards_sigterm_to_the_running_child() {
    // Without forwarding the child would sleep out its full 10 seconds (or
    // the test process would die outright); with it the child exits promptly
    // with the signal-death code 128 + 15.
    let started = std::time::Instant::now();
    let handle = std::thread::spawn(|| {
        let mut runner = SystemRunner::new();
        runner
            .run(&Invocation::new(["/bin/sh", "-c", "sleep 10"], "/"))
            .unwrap()
    });
    // Give the runner time to install the handler and spawn the child.
    std::thread::sleep(std::time::Duration::from_millis(300));

    unsafe { kill(getpid(), SIGTERM) };

    let outcome = handle.join().unwrap();
    assert_eq!(outcome.exit_code, 128 + SIGTERM);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(8),
        "the child was killed, not waited out"
    );
}
