extern crate liars;
extern crate rand;
extern crate tokio_test;

use rand::Rng;

use liars::play::PlayArgs;
use liars::playexpert::PlayExpertArgs;
use liars::start::*;

#[test]
fn test() {
    tokio_test::block_on(test_impl());
}

/// Test with a full quorum.
async fn test_impl() {
    for _ in 0..10 {
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
        struct ProcessCleanup(Vec<std::process::Child>);
        impl Drop for ProcessCleanup {
            fn drop(&mut self) {
                for child in &mut self.0 {
                    let _ = child.kill();
                }
                let _ = std::fs::remove_file("agents.conf");
            }
        }
        start(&start_args).await;

        // Test that `play` works.
        let play_args = PlayArgs {
            path: std::path::PathBuf::from("agents.conf"),
        };
        let result = liars::play::play(&play_args).await;
        assert_eq!(result.expect("We should have a result"), value, "'play' should produce the right value");

        // Test that `playexpert` works.
        let play_expert_args = PlayExpertArgs {
            path: std::path::PathBuf::from("agents.conf"),
            liar_ratio,
        };
        let result = liars::playexpert::play(&play_expert_args).await;
        assert_eq!(result.expect("We should have a result"), value, "'playexpert' should produce the right value");
    }
}