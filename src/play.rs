use std::path::PathBuf;

use crate::agent;
use crate::conf::*;

pub struct PlayArgs {
    pub path: PathBuf,
}

pub async fn play(args: &PlayArgs) {
    // Attempt to parse configuration.
    let file = std::fs::File::open(&args.path).expect("Could not open file");
    let conf: Conf = serde_json::from_reader(&file).expect("Could not read file");

    // Talk to each agent.
    // We're in no hurry, do it sequentially.
    let mut yeas = 0usize;
    let mut nays = 0usize;
    let mut result = None;

    for child in &conf.children {
        let remote = agent::RemoteAgent::new(child.clone());
        match remote.call(&agent::Message::GetValue).await {
            Ok(agent::Response::Value(true)) => {
                yeas += 1;
            }
            Ok(agent::Response::Value(false)) => {
                nays += 1;
            }
            Err(error) => {
                eprintln!("Could not communicate with child {pid} on port {port}: {error:?}, skipping child.",
                    pid = child.pid,
                    port = child.socket,
                    error = error
                );
            }
        }

        if yeas >= conf.children.len() / 2 {
            // We have a quorum, no need to proceed.
            result = Some(true);
            break;
        } else if nays >= conf.children.len() / 2 {
            // We have a quorum, no need to proceed.
            result = Some(false);
            break;
        }
    }

    match result {
        Some(true) => eprintln!("The value was 'true'"),
        Some(false) => eprintln!("The value was 'false'"),
        None => eprintln!("Not enough participants to determine value"),
    }
}
