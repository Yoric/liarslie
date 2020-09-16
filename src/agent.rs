use std::io::{ BufRead, BufReader, LineWriter, Write };
use std::net::{ SocketAddr, TcpListener };

use serde_derive::{ Serialize, Deserialize };
use crate::conf::Child;


#[derive(Debug, Deserialize, Serialize)]
pub enum Message {
    GetValue,
}
#[derive(Debug, Deserialize, Serialize)]
pub enum Response {
    Value(bool)
}

/// An agent running in this process
pub struct Agent {
    value: bool,
    listener: TcpListener,
}
impl Agent {
    /// Create an agent, open a socket.
    pub fn try_new(value: bool) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        Ok(Agent {
            value,
            listener
        })
    }
    pub fn socket(&self) -> SocketAddr {
        self.listener.local_addr()
            .expect("No local address")
    }

    /// Enter the loop, forever.
    pub fn exec(&mut self) {
        loop {
            // Wait for a connection.
            eprintln!("Agent: waiting for connection on port {}", self.socket().port());
            let (mut conn, _) = self.listener.accept()
                .expect("Could not accept connection");

            // Process requests.
            // For the time being, we only process requests from a single source at a time.
            let mut reader = BufReader::new(&mut conn);
            'lines: loop {
                eprintln!("Agent: received connection");
                // Receive message.
                let mut line = String::new();
                let line = match reader.read_line(&mut line) {
                    Ok(0) => {
                        eprintln!("Agent: connection closed by remote host");
                        break 'lines
                    }
                    Ok(_) => line,
                    Err(err) => {
                        eprintln!("Could not read, closing connection {:?}.", err);
                        break 'lines;
                    }
                };

                eprintln!("Agent: received message '{}'", line);
                let message = match serde_json::from_str(&line) {
                    Err(err) => {
                        eprintln!("Invalid message, closing connection {:?}.", err);
                        break 'lines;
                    }
                    Ok(msg) => msg,
                };

                eprintln!("Agent: message is correct, preparing response");

                // And respond
                let mut writer = LineWriter::new(reader.get_mut());
                let response = match message {
                    Message::GetValue => {
                        Response::Value(self.value)
                    }
                };

                if let Err(err) = serde_json::to_writer(&mut writer, &response) {
                    eprintln!("Could not respond, closing connection {:?}.", err);
                    break 'lines;
                }
                if let Err(err) = writer.write_all(b"\n") {
                    eprintln!("Could not respond, closing connection {:?}.", err);
                    break 'lines;
                }
            }
        }
    }
}

/// An agent running in another process.
pub struct RemoteAgent {
    conf: Child,
}
impl RemoteAgent {
    pub fn new(conf: Child) -> Self  {
        RemoteAgent {
            conf
        }
    }
}

pub struct AgentArgs {
    pub value: bool,
}

/// Start agent, print port on stdout, enter agent main loop, never return.
pub fn agent(args: &AgentArgs) {
    let mut agent = Agent::try_new(args.value)
        .expect("Could not start agent");
    print!("{}\n", agent.socket().port());
    agent.exec();
    unreachable!();
}