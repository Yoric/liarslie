use std::io::{ Read, Write };
use std::net::{ SocketAddr, TcpListener };

use enum_ordinalize::Ordinalize;

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Ordinalize)]
pub enum Message {
    GetValue,
}

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

            let mut buf = [0; 1];
            conn.read_exact(&mut buf)
                .expect("Could not receive data");

            match Message::from_ordinal(buf[0]) {
                Some(Message::GetValue) => {
                    // Provide value.
                    conn.write_all(&[ if self.value { 1 } else { 0 }])
                        .expect("Could not write to socket");
                }
                None => {
                    eprintln!("Received bad message {:?}", buf);
                    continue;
                }
            }
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