use std::io::{ BufRead, BufReader, Read, Write };
use std::net::TcpStream;
use std::path::PathBuf;

use crate::agent;
use crate::conf::*;

pub struct PlayArgs {
    pub path: PathBuf,
}

pub fn play(args: &PlayArgs) {
    // Attempt to parse configuration.
    let file = std::fs::File::open(&args.path)
        .expect("Could not open file");
    let conf : Conf = serde_json::from_reader(&file)
        .expect("Could not read file");

    // Talk to each agent.
    // We're in no hurry, do it sequentially.
    let mut yeas = 0usize;
    let mut nays = 0usize;
    let mut result = None;
    let comm = |child: &Child| -> Result<bool, std::io::Error> {
        eprintln!("Play: Connecting with child {pid} on port {port}",
            port = child.socket,
            pid = child.pid);
        // Acquire child.
        let mut stream = TcpStream::connect(std::net::SocketAddr::from(([127, 0, 0, 1], child.socket)))?;

        // Adopt arbitrary timeouts.
        stream.set_write_timeout(Some(std::time::Duration::new(5, 0)))
            .unwrap();
        stream.set_read_timeout(Some(std::time::Duration::new(5, 0)))
            .unwrap();

        // Send request.
        eprintln!("Play: Sending request");
        serde_json::to_writer(&mut stream, &agent::Message::GetValue)?;
        stream.write_all(b"\n")?;
        stream.flush()?;

        // Wait for response.
        eprintln!("Play: Waiting for response");
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        match serde_json::from_str(&line)? {
            agent::Response::Value(v) => Ok(v)
        }
    };

    for child in &conf.children {
        match comm(child) {
            Ok(true) => {
                yeas += 1;
            }
            Ok(false) => {
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
        None => eprintln!("Not enough participants to determine value")
    }
}