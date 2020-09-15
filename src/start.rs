use crate::conf::*;

pub struct StartArgs {
    pub value: bool,
    pub num_agents: usize,
    pub liar_ratio: f64,
}

/// Implementation of command `start`.
///
/// Start `args.num_agents` processes with `args.liar_ratio` liars.
pub fn start(args: &StartArgs) {
    use crate::rand::prelude::SliceRandom;
    use std::io::{ BufRead, BufReader, Write };
    let num_liars = ((args.num_agents as f64) * args.liar_ratio) as usize;

    // Initialize the values we're about to distribute among agents.
    // Initially, everybody is a reliable.
    let mut values = Vec::with_capacity(args.num_agents);
    for _ in 0 .. args.num_agents {
        values.push(args.value);
    }
    // Introduce exactly `num_liars` liars.
    for i in 0..num_liars {
        values[i] = !args.value;
    }
    values.shuffle(&mut rand::thread_rng());

    // Spawn agents.
    let exe = std::env::current_exe()
        .expect("Could not get executable");
    let mut processes= Vec::with_capacity(args.num_agents);
    for v in values {
        let mut cmd = std::process::Command::new(&exe);
        let child = cmd.arg("agent")
            .arg("--value")
            .arg(if v { "true" } else { "false" })
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Could not spawn process");
        eprintln!("Launching command {:?}", cmd);
        processes.push(child);
    }

    // Wait to know their socket. We're in no hurry here, so let's do it sequentially.
    let mut children = Vec::with_capacity(args.num_agents);
    for mut proc in processes {
        let stdout = proc.stdout.as_mut()
            .expect("Could not access child process stdout");
        let mut reader = BufReader::new(stdout);
        let mut received = String::new();
        reader.read_line(&mut received)
            .expect("Could not communicate with child process");
        let socket = received[0..received.len() - 1].parse::<u16>()
            .expect("Did not receive a socket");
        children.push(Child {
            pid: proc.id(),
            socket,
        });
    }

    eprintln!("Value is {}, spawned {} processes, {} of which are lying.\n{:?}",
        args.value,
        args.num_agents,
        num_liars,
        children
    );

    // Write `agents.conf`
    {
        let config = Conf {
            children
        };
        let serialized = serde_json::to_string_pretty(&config)
            .unwrap();
        let mut file = std::fs::File::create("agents.conf")
            .expect("Cannot create agents.conf");
        write!(file, "{}", serialized)
            .expect("Cannot write agents.conf");
    }

    eprintln!("Ready");
}
