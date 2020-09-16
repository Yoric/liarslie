use std::net::SocketAddr;
use tokio::io::{ AsyncBufReadExt, BufReader, AsyncWriteExt };
use tokio::net::TcpListener;

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
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        Ok(Agent {
            value,
            listener: TcpListener::from_std(listener)
                .unwrap(),
        })
    }
    pub fn socket(&self) -> SocketAddr {
        self.listener.local_addr()
            .expect("No local address")
    }

    /// Enter the loop, forever.
    pub async fn exec(&mut self) {
        loop {
            // Wait for a connection.
            eprintln!("Agent: waiting for connection on port {}", self.socket().port());
            let (mut conn, _) = self.listener.accept().await
                .expect("Could not accept connection");

            let value = self.value;
            tokio::spawn(async move {
                // Process requests.
                let mut reader = BufReader::new(&mut conn);
                'lines: loop {
                    eprintln!("Agent: received connection");
                    // Receive message.
                    let mut line = String::new();
                    let line = match reader.read_line(&mut line).await {
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
                    let response = match message {
                        Message::GetValue => {
                            Response::Value(value)
                        }
                    };
                    let mut response = serde_json::to_string(&response)
                        .unwrap();
                    response.push('\n');
                    if let Err(err) = reader.get_mut().write_all(response.as_bytes()).await {
                        eprintln!("Could not respond, closing connection {:?}.", err);
                        break 'lines;
                    }
                }
            });
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
pub async fn agent(args: &AgentArgs) {
    let mut agent = Agent::try_new(args.value)
        .expect("Could not start agent");
    print!("{}\n", agent.socket().port());
    agent.exec().await;
    unreachable!();
}