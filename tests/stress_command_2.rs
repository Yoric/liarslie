// Attempt to stress tokio::process::Command::spawn
// to reproduce a `WouldBlock` error.
#[test]
fn main() {
    env_logger::init();
    tokio_test::block_on(async {
        // A little time to launch dtruss.
        // The error shows up even without this wait.
        tokio::time::delay_for(std::time::Duration::new(3, 0)).await;

        let mut cmd = tokio::process::Command::new("ls");
        for i in 0..10000 {
            eprintln!("Process {}", i);
            let mut child = cmd.spawn().expect("Could not spawn process");
            child.kill().expect("Could not kill process");
            child
                .wait_with_output()
                .await
                .expect("Could not wait_with_output");
        }
    });
}
