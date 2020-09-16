use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use crate::conf::Child;
use serde_derive::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Deserialize, Serialize)]
pub enum Message {
    /// Get the value carried by this agent.
    ///
    /// Response is `Response::Certificate(Certificate)`.
    GetValue,

    /// Request a list of allies for this agent.
    ///
    /// Response is `Response::Quorum(...)`.
    Campaign(Vec<Child>),
}
#[derive(Debug, Deserialize, Serialize)]
pub enum Response {
    Certificate(Certificate),
    Quorum(Vec<Certificate>),
}

/// Representation of an unforgeable response.
///
/// In an actual implementation, this could either be
/// - backed by cryptography; or
/// - backed by double-checking with the agent that they have issued this response.
#[derive(Debug, Deserialize, Serialize)]
pub struct Certificate {
    pub value: bool,
    pub issuer: Child,
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
            listener: TcpListener::from_std(listener).unwrap(),
        })
    }
    pub fn socket(&self) -> SocketAddr {
        self.listener.local_addr().expect("No local address")
    }

    /// Enter the loop, forever.
    pub async fn exec(&mut self) {
        let value = self.value;
        let issuer = Child {
            socket: self.socket().port(),
            pid: std::process::id(),
        };
        loop {
            // Wait for a connection.
            eprintln!(
                "Agent: waiting for connection on port {}",
                self.socket().port()
            );
            let (mut conn, _) = self
                .listener
                .accept()
                .await
                .expect("Could not accept connection");

            let issuer = issuer.clone();
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
                            break 'lines;
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

                    // And respond.
                    let response = match message {
                        Message::GetValue => Response::Certificate(Certificate {
                            value,
                            issuer: issuer.clone(),
                        }),
                        Message::Campaign(children) => {
                            let (tcollect, mut rcollect) = tokio::sync::mpsc::channel(32);
                            let collector = tokio::spawn(async move {
                                let mut my_party = vec![];
                                while let Some(certificate) = rcollect.recv().await {
                                    my_party.push(certificate);
                                }
                                my_party
                            });
                            {
                                // Make sure that `tcollect` is dropped after the async loop is over.
                                let tcollect = tcollect;
                                for child in children {
                                    let mut tcollect = tcollect.clone();
                                    // We could of course avoid calling ourself.
                                    // Let's see this as a stress-test for concurrency/reentrancy issues!
                                    let remote = RemoteAgent::new(child);
                                    tokio::spawn(async move {
                                        match remote.call(&Message::GetValue).await {
                                            Ok(Response::Certificate(certificate)) => {
                                                if certificate.value != value {
                                                    // Remote agent disagrees with us, ignore it.
                                                    return;
                                                }
                                                let _ = tcollect.send(certificate).await;
                                            }
                                            _ => {
                                                // Remote agent can't or won't respond or bad response, skip it.
                                            }
                                        }
                                    });
                                }
                            }
                            Response::Quorum(collector.await.unwrap())
                        }
                    };
                    let mut response = serde_json::to_string(&response).unwrap();
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
    pub fn new(conf: Child) -> Self {
        RemoteAgent { conf }
    }
    pub async fn call(&self, message: &Message) -> Result<Response, std::io::Error> {
        eprintln!(
            "Play: Connecting with child {pid} on port {port}",
            port = self.conf.socket,
            pid = self.conf.pid
        );
        // Acquire child.
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", self.conf.socket)).await?;

        // Send request.
        eprintln!("Play: Sending request");
        let mut buffer = serde_json::to_string(message).unwrap();
        buffer.push('\n');
        stream.write_all(buffer.as_bytes()).await?;
        stream.flush().await?;

        // Wait for response.
        eprintln!("Play: Waiting for response");
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        Ok(serde_json::from_str(&line)?)
    }
}

pub struct AgentArgs {
    pub value: bool,
}

/// Start agent, print port on stdout, enter agent main loop, never return.
pub async fn agent(args: &AgentArgs) {
    let mut agent = Agent::try_new(args.value).expect("Could not start agent");
    print!("{}\n", agent.socket().port());
    agent.exec().await;
    unreachable!();
}
