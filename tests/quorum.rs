extern crate liars;
extern crate rand;
extern crate tokio_test;

use rand::Rng;

use liars::play::PlayArgs;
use liars::playexpert::PlayExpertArgs;
use liars::start::*;

struct ProcessCleanup {
    processes: Vec<tokio::process::Child>,
}
impl Drop for ProcessCleanup {
    fn drop(&mut self) {
        for child in &mut self.processes {
            let _ = child.kill().unwrap();
        }
        let _ = std::fs::remove_file("agents.conf");
    }
}

#[test]
fn test() {
    env_logger::init();
    tokio_test::block_on(test_impl());
}

/// Test with a full quorum.
async fn test_impl() {
    for i in 0..100 {
        eprintln!("Initializing test {}", i);

        // Start with processes.
        let value = rand::thread_rng().gen_bool(0.5);
        let liar_ratio = rand::thread_rng().gen_range(0.0, 0.5);
        let num_agents = rand::thread_rng().gen_range(10, 50);
        let exe = std::path::PathBuf::from(env!("CARGO_BIN_EXE_liarslie"));
        let start_args = StartArgs {
            value,
            liar_ratio,
            num_agents,
            exe,
        };
        // Cleanup processes on exit.
        let (conf, processes) = start(&start_args).await;
        let _guard = ProcessCleanup { processes };
        assert_eq!(_guard.processes.len(), num_agents);

        // Alternate play and expert runs in a random order, as a stress test.
        let mut play_runs = 0;
        let mut expert_runs = 0;
        while play_runs < 5 || expert_runs < 5 {
            if rand::thread_rng().gen_bool(0.5) {
                play_runs += 1;
                // Test that `play` provides the right result.
                eprintln!("...Testing play in this configuration");
                let play_args = PlayArgs {
                    path: std::path::PathBuf::from("agents.conf"),
                };
                let result = liars::play::play(&play_args).await;
                assert_eq!(
                    result.expect("We should have a result"),
                    value,
                    "'play' should produce the right value"
                );
            } else {
                expert_runs += 1;
                // Test that `playexpert` provides the right result.
                eprintln!("...Testing playexpert in this configuration");
                let play_expert_args = PlayExpertArgs {
                    path: std::path::PathBuf::from("agents.conf"),
                    liar_ratio,
                };
                let result = liars::playexpert::play(&play_expert_args).await;
                assert_eq!(
                    result.expect("We should have a result"),
                    value,
                    "'playexpert' should produce the right value"
                );
            }
        }

        // Try to close sockets
        for child in &conf.children {
            let remote = liars::agent::RemoteAgent::new(child.clone());
            let _ = remote.call(&liars::agent::Message::Stop).await;
        }
    }
}
