use std::path::PathBuf;

use log::*;

use crate::conf::*;
use crate::util;

pub struct StartArgs {
    pub exe: PathBuf,
    pub value: bool,
    pub num_agents: usize,
    pub liar_ratio: f64,
}

/// Implementation of command `start`.
///
/// Start `args.num_agents` processes with `args.liar_ratio` liars.
pub async fn start(args: &StartArgs) -> (Conf, Vec<tokio::process::Child>) {
    use crate::rand::prelude::SliceRandom;
    use std::io::Write;
    use tokio::io::{AsyncBufReadExt, BufReader};
    let num_liars = ((args.num_agents as f64) * args.liar_ratio) as usize;
    debug!(target: "start", "Preparing {} agents including {} liars",
        args.num_agents,
        num_liars);

    // Initialize the values we're about to distribute among agents.
    // Initially, everybody is a reliable.
    let mut values = Vec::with_capacity(args.num_agents);
    for _ in 0..args.num_agents {
        values.push(args.value);
    }
    // Introduce exactly `num_liars` liars.
    for i in 0..num_liars {
        values[i] = !args.value;
    }
    values.shuffle(&mut rand::thread_rng());

    // Spawn agents.
    let mut processes = Vec::with_capacity(args.num_agents);
    for v in values {
        let mut cmd = tokio::process::Command::new(&args.exe);
        cmd.arg("agent")
            .arg("--value")
            .arg(if v { "true" } else { "false" })
            .stdout(std::process::Stdio::piped());

        // We may need several attempts to spawn processes, if the machine is a bit stressed.
        let child = util::retry_closure_if(
            move || cmd.spawn(),
            |err| err.kind() == std::io::ErrorKind::WouldBlock,
        )
        .await
        .expect("Could not spawn process");
        processes.push(child);
    }

    // Wait to know their socket. We're in no hurry here, so let's do it sequentially.
    let mut children = Vec::with_capacity(args.num_agents);
    for proc in &mut processes {
        let stdout = proc
            .stdout
            .as_mut()
            .expect("Could not access child process stdout");
        let mut reader = BufReader::new(stdout);
        let mut received = String::new();
        reader
            .read_line(&mut received)
            .await
            .expect("Could not communicate with child process");
        let socket = received[0..received.len() - 1]
            .parse::<u16>()
            .expect("Did not receive a socket");
        children.push(Child {
            pid: proc.id(),
            socket,
        });
    }

    debug!(target: "start",
        "Value is {}, spawned {} processes, {} of which are lying.\n{:?}",
        args.value, args.num_agents, num_liars, children
    );

    // Write `agents.conf`
    let config = Conf { children };
    let serialized = serde_json::to_string_pretty(&config).unwrap();
    let mut file = std::fs::File::create("agents.conf").expect("Cannot create agents.conf");
    write!(file, "{}", serialized).expect("Cannot write agents.conf");

    debug!(target: "start", "Ready");

    (config, processes)
}
