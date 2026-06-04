//! [`ClientConnection`] — a ZMQ REQ socket connection from a debug client to the debug server
//! in the coordinator.

use std::fmt::Display;

use log::{info, trace};
use zmq::Socket;

use flowcore::errors::{Result, ResultExt};

/// `ClientConnection` stores information related to the connection from a debug client
/// to the debug server in the [Coordinator][flowrlib::coordinator::Coordinator].
pub struct ClientConnection {
    requester: Socket,
}

impl ClientConnection {
    /// Create a new connection to a debug server at the given address
    ///
    /// # Errors
    /// Returns an error if the ZMQ socket cannot be created or connected
    pub fn new(server_address: &str) -> Result<Self> {
        info!("Debug client will attempt to connect to server at: '{server_address}'");

        let context = zmq::Context::new();

        let requester = context
            .socket(zmq::REQ)
            .chain_err(|| "Debug client could not create REQ socket")?;

        requester
            .connect(&format!("tcp://{server_address}"))
            .chain_err(|| {
                format!("Debug client could not connect to socket at: {server_address}")
            })?;

        info!("Debug client connected to server at '{server_address}'");

        Ok(ClientConnection { requester })
    }

    /// Receive a message from the debug server
    ///
    /// # Errors
    /// Returns an error if the message cannot be received or deserialized
    pub fn receive<CM>(&self) -> Result<CM>
    where
        CM: From<String> + Display,
    {
        trace!("Debug client waiting for message from server");

        let msg = self
            .requester
            .recv_msg(0)
            .map_err(|e| format!("Error receiving from debug server: {e}"))?;

        let message_string = msg
            .as_str()
            .ok_or("Could not get message as str")?
            .to_string();
        let message: CM = message_string.into();
        trace!("Debug client received <--- {message}");
        Ok(message)
    }

    /// Send a message to the debug server
    ///
    /// # Errors
    /// Returns an error if the message cannot be sent
    pub fn send<CM>(&self, message: CM) -> Result<()>
    where
        CM: Into<String> + Display,
    {
        trace!("Debug client sent     ---> {message}");
        self.requester
            .send(&message.into(), 0)
            .chain_err(|| "Error sending to debug server")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use std::fmt;

    use portpicker::pick_unused_port;
    use serde_derive::{Deserialize, Serialize};
    use serial_test::serial;

    use super::ClientConnection;
    use crate::coordinator_connection::{CoordinatorConnection, WAIT};

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    enum Reply {
        Ok,
    }

    impl fmt::Display for Reply {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "Reply::Ok")
        }
    }

    impl From<Reply> for String {
        fn from(r: Reply) -> Self {
            serde_json::to_string(&r).expect("serialize")
        }
    }

    impl From<String> for Reply {
        fn from(s: String) -> Self {
            serde_json::from_str(&s).expect("deserialize")
        }
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    enum Cmd {
        Ping,
    }

    impl fmt::Display for Cmd {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "Cmd::Ping")
        }
    }

    impl From<Cmd> for String {
        fn from(c: Cmd) -> Self {
            serde_json::to_string(&c).expect("serialize")
        }
    }

    impl From<String> for Cmd {
        fn from(s: String) -> Self {
            serde_json::from_str(&s).expect("deserialize")
        }
    }

    #[test]
    #[serial]
    fn client_send_receive_roundtrip() {
        let port = pick_unused_port().expect("No ports free");
        let mut server = CoordinatorConnection::new(&format!("test-{port}"), port).expect("bind");
        let client = ClientConnection::new(&format!("localhost:{port}")).expect("connect");

        client.send(Cmd::Ping).expect("send");

        let cmd: Cmd = server.receive(WAIT).expect("receive");
        assert_eq!(cmd, Cmd::Ping);

        server.send(Reply::Ok).expect("reply");

        let reply: Reply = client.receive().expect("receive reply");
        assert_eq!(reply, Reply::Ok);
    }
}
