use std::io::{ Read, Write };
use std::net::TcpStream;
use std::path::PathBuf;

use enum_ordinalize::Ordinalize;

use crate::agent;
use crate::conf::*;

pub struct PlayArgs {
    pub path: PathBuf,
}

pub fn play(args: &PlayArgs) {
    // Attempt to parse configuration.
    let mut file = std::fs::File::open(&args.path)
        .expect("Could not open file");
    let conf : Conf = serde_json::from_reader(&file)
        .expect("Could not read file");

    // Talk to each agent.
    // We're in no hurry, do it sequentially.
    let mut yeas = 0usize;
    let mut nays = 0usize;
    let mut result = None;
    for child in &conf.children {
        eprintln!("Connecting with child {pid} on port {port}",
            port = child.socket,
            pid = child.pid);
        // Acquire child.
        let mut stream = match TcpStream::connect(std::net::SocketAddr::from(([127, 0, 0, 1], child.socket))) {
            Err(err) => {
                // In case the process is dead, attempt to continue with remaining children.
                eprintln!("Could not connect with child process {pid} on port {port}: {err:?}",
                    port = child.socket,
                    pid = child.pid,
                    err = err);
                    continue;
            }
            Ok(stream) => stream
        };

        // Send message.
        stream.write_all(&mut [agent::Message::GetValue.ordinal()])
            .expect("Could not send request to agent");

        // Wait for response.
        // At this stage, we expect that the child will respond promptly, even if that's
        // not necessarily realistic.
        let mut response = [255];
        stream.read_exact(&mut response)
            .expect("Could not receive value from agent");

        let claim = match response[0] {
            0 => false,
            1 => true,
            other => panic!("Unexpected response {}", other)
        };
        if claim {
            yeas += 1;
        } else {
            nays += 1;
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
        None => eprintln!("Not enough participants to determine value")
    }
}