use std::iter::Iterator;
use std::path::PathBuf;

use log::*;
use rand::seq::SliceRandom;

use crate::agent;
use crate::conf::*;

pub struct PlayExpertArgs {
    pub liar_ratio: f64,
    pub path: PathBuf,
}

pub async fn play(args: &PlayExpertArgs) -> Option<bool> {
    // Attempt to parse configuration.
    let file = std::fs::File::open(&args.path).expect("Could not open file");
    let conf: Conf = serde_json::from_reader(&file).expect("Could not read file");
    let number_of_children = conf.children.len();
    let (tcollect, mut rcollect) = tokio::sync::mpsc::channel::<Vec<agent::Certificate>>(32);

    // Collect responses.
    let collector = tokio::spawn(async move {
        debug!(target: "playexpert", "Starting");
        while let Some(party) = rcollect.recv().await {
            debug!(target: "playexpert", "Received a party of {} certificates (from {} processes)", party.len(), number_of_children);
            if party.len() < number_of_children / 2 {
                // The party is too small to be a quorum, ignore.
                debug!(target: "playexpert", "Party is too small to be a quorum");
                continue;
            }
            // Let's check that the quorum *is* a quorum.
            // FIXME: This is where we should check that the messages haven't been forged
            // and/or double-check with issuer.
            let (yeas, nays): (Vec<_>, Vec<_>) =
                party.into_iter().partition(|certificate| certificate.value);
            if yeas.len() >= (number_of_children + 1) / 2 {
                debug!(target: "playexpert", "got {} voters for yea that's a quorum", yeas.len());
                return Some(true);
            }
            if nays.len() >= (number_of_children + 1) / 2 {
                debug!(target: "playexpert", "got {} voters for nay that's a quorum", nays.len());
                return Some(false);
            }
        }
        debug!(target: "playexpert", "Done");
        None
    });

    // Pick a number of agents and talk to them.
    let tasks: Vec<_> = {
        // Make sure that `tcollect` is fully dropped once all tasks are complete.
        let tcollect = tcollect;
        let children = conf.children.clone();
        let number_of_interlocutors =
            (number_of_children as f64 * (1.0 - args.liar_ratio)) as usize + 1;
        let interlocutors = conf
            .children
            .choose_multiple(&mut rand::thread_rng(), number_of_interlocutors);
        interlocutors.cloned().map(|child| {
            let children = children.clone();
            let remote = agent::RemoteAgent::new(child.clone());
            let mut tcollect = tcollect.clone();
            tokio::spawn(async move {
                match remote.call(&agent::Message::Campaign(children)).await {
                    Ok(agent::Response::Quorum(party)) => {
                        // Ignore errors: the collector may have finished already.
                        let _ = tcollect.send(party).await;
                    }
                    Ok(other) => {
                        debug!(target: "playexpert", "Bad response from child {pid} on port {port}: {response:?}",
                            pid = child.pid,
                            port = child.socket,
                            response = other
                        );
                    }
                    Err(error) => {
                        debug!(target: "playexpert", "Could not communicate with child {pid} on port {port}: {error:?}, skipping child.",
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
    let result =collector.await.unwrap();
    match result {
        Some(true) => debug!(target: "playexpert", "The value was 'true'"),
        Some(false) => debug!(target: "playexpert", "The value was 'false'"),
        None => debug!(target: "playexpert", "Not enough participants to determine value"),
    };
    result
}