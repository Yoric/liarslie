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
    let number_of_children = conf.children.len();
    let (tcollect, mut rcollect) = tokio::sync::mpsc::channel(32);

    // Collect responses.
    let collector = tokio::spawn(async move {
        let mut yeas = 0usize;
        let mut nays = 0usize;
        let mut result = None;
        eprintln!("Collector: Starting");
        while let Some(msg) = rcollect.recv().await {
            eprintln!("Collector: Treating {}", msg);
            if msg {
                yeas += 1;
                if yeas >= number_of_children / 2 {
                    // We have a quorum, no need to proceed.
                    result = Some(true);
                    break;
                }
            } else {
                nays += 1;
                if nays >= number_of_children / 2 {
                    // We have a quorum, no need to proceed.
                    result = Some(false);
                    break;
                }
            }
            eprintln!("Collector: yeas {}, nays {}, we should continue", yeas, nays);
        }
        eprintln!("Collector: Done");
        return result;
    });

    // Talk to each agent.
    let tasks: Vec<_> = {
        // Make sure that `tcollect` is fully dropped once all tasks are complete.
        let tcollect = tcollect;
        conf.children.iter().cloned().map(|child| {
            let remote = agent::RemoteAgent::new(child.clone());
            let mut tcollect = tcollect.clone();
            tokio::spawn(async move {
                match remote.call(&agent::Message::GetValue).await {
                    Ok(agent::Response::Certificate(agent::Certificate { value, .. })) => {
                        eprintln!("Play: Received value {} from remote agent", value);
                        // Ignore errors: the collector may have finished already.
                        let _ = tcollect.send(value).await;
                    }
                    Ok(other) => {
                        eprintln!("Bad response from child {pid} on port {port}: {response:?}",
                            pid = child.pid,
                            port = child.socket,
                            response = other
                        );
                    }
                    Err(error) => {
                        eprintln!("Could not communicate with child {pid} on port {port}: {error:?}, skipping child.",
                            pid = child.pid,
                            port = child.socket,
                            error = error
                        );
                    }
                }
            })
        }).collect()
    };

    for task in tasks.into_iter() {
        task.await.unwrap();
    }
    match collector.await.unwrap() {
        Some(true) => eprintln!("The value was 'true'"),
        Some(false) => eprintln!("The value was 'false'"),
        None => eprintln!("Not enough participants to determine value"),
    }
}
