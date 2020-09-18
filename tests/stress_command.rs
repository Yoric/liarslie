const MAX_TRIES: u64 = 1;

// Attempt to stress tokio::process::Command::spawn
// to reproduce a `WouldBlock` error.
#[test]
fn main() {
    env_logger::init();
    tokio_test::block_on(async {
        // A little time to launch dtruss
        tokio::time::delay_for(std::time::Duration::new(3, 0)).await;

        let mut cmd = tokio::process::Command::new("/bin/ls");
        cmd.kill_on_drop(true) // Cleanup at the end of the test
            .stdout(std::process::Stdio::piped());
        // let mut processes = vec![];
        for i in 0..10000 {
            eprintln!("Process {}", i);
            let mut error = None;
            for i in 0..MAX_TRIES {
                match cmd.spawn() {
                    Ok(mut process) => {
                        let _ = process.kill();
                        error = None;
                        break;
                    }
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::WouldBlock {
                            error = Some(err);
                            if i + 1 < MAX_TRIES {
                                tokio::time::delay_for(std::time::Duration::new(
                                    (i + 1) * (i + 1),
                                    0,
                                ))
                                .await;
                            }
                            continue;
                        }
                        panic!("Could not spawn process: {:?}", err);
                    }
                }
            }
            if let Some(err) = error {
                panic!(
                    "Could not spawn process after {} retries: {:?}",
                    MAX_TRIES, err
                );
            }
        }
    });
}
