use log::{info, trace};
use serde::ser::Serialize;
use serde::Deserialize;
/// This is the channel-based implementation of the lib.client_server communications
use std::fmt::Debug;
use std::fmt::Display;
use std::marker::PhantomData;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::errors::*;

/// `ClientConnection` stores information related to the connection from a runtime client
/// to the runtime server and is used each time a message is to be sent or received.
pub struct ClientConnection<'a, SM, CM>
where
    SM: Serialize,
    CM: Deserialize<'a>,
{
    channels: (Arc<Mutex<Receiver<SM>>>, Sender<CM>),
    phantom: PhantomData<&'a SM>,
}

impl<'a, SM, CM> ClientConnection<'a, SM, CM>
where
    SM: Serialize + Display,
    CM: Deserialize<'a> + Display,
{
    /// Create a new connection between client and server
    pub fn new(_server_hostname: &Option<String>, _port: usize) -> Result<Self> {
        info!("Client connection (channels transport) created");

        Ok(ClientConnection {
            channels: runtime_server_connection.get_channels(),
            phantom: PhantomData,
        })
    }

    /// Receive a Message from the server
    pub fn receive(&self) -> Result<SM> {
        let guard = self
            .channels
            .0
            .lock()
            .map_err(|_| "Could not lock client message reception channel")?;

        trace!("Client waiting for message from server");

        let message = guard
            .recv()
            .chain_err(|| "Error receiving message from client channel")?;

        trace!("Client Received <--- {}", message);
        Ok(message)
    }

    /// Send a Message from the runtime client to the runtime server
    pub fn send(&self, message: CM) -> Result<()> {
        trace!("Client Sent     ---> {}", message);
        self.channels
            .1
            .send(message)
            .map_err(|_| "Error sending on client channel")?;

        Ok(())
    }
}

/// `ServerConnection` store information about the server side of the client/server
/// communications between a client and a server and is used each time a message
/// needs to be sent or received.
#[derive(Debug)]
pub struct ServerConnection<SM, CM> {
    /// A channel for the server to send server messages to a client on
    server_tx: Sender<SM>,
    /// A channel for a client to receive server messages on
    client_rx: Arc<Mutex<Receiver<SM>>>,
    /// A channel to for a client to send client messages to the server on
    client_tx: Sender<CM>,
    /// A channel where server will receive client message from a client on
    server_rx: Receiver<CM>,
}

impl<'a, SM, CM> ServerConnection<SM, CM>
where
    SM: Serialize + Display,
    CM: Deserialize<'a> + Display,
{
    /// Create a new Server side of the client/server Connection
    pub fn new(_server_hostname: &Option<String>, _port: usize) -> Result<Self> {
        let (client_event_channel_tx, client_event_channel_rx) = mpsc::channel();
        let (client_response_channel_tx, client_response_channel_rx) = mpsc::channel();

        trace!("Server Connection (channels transport) created");
        Ok(ServerConnection {
            server_tx: client_event_channel_tx,
            client_rx: Arc::new(Mutex::new(client_event_channel_rx)),
            client_tx: client_response_channel_tx,
            server_rx: client_response_channel_rx,
        })
    }

    /// Get the channels a client should use to send to the server
    fn get_channels(&self) -> (Arc<Mutex<Receiver<SM>>>, Sender<CM>) {
        // Clone of Arc and Sender is OK
        (self.client_rx.clone(), self.client_tx.clone())
    }

    /// Get a Message sent to the client from the server
    pub fn receive(&self) -> Result<CM> {
        trace!("Server waiting for message from client");

        let message = self
            .server_rx
            .recv()
            .chain_err(|| "Error receiving response from client")?;
        trace!("                ---> Server Received {}", message);
        Ok(message)
    }

    /// Try to get a Message sent to the client to the server but without blocking
    pub fn receive_no_wait(&self) -> Result<CM> {
        let message = self
            .server_rx
            .try_recv()
            .chain_err(|| "Error receiving response from client")?;
        trace!("                ---> Server Received {}", message);
        Ok(message)
    }

    /// Send a server Message to the client and wait for it's response
    pub fn send_and_receive_response(&mut self, message: SM) -> Result<CM> {
        self.send(message)?;
        self.receive()
    }

    /// Send a server Message to the client but don't wait for it's response
    pub fn send(&mut self, message: SM) -> Result<()> {
        trace!("                <--- Server Sent {}", message);
        self.server_tx
            .send(message)
            .map_err(|e| format!("Error sending to client: '{}'", e))?;

        Ok(())
    }

    /// Close the Server side of the client/server Connection
    pub fn close(&mut self) -> Result<()> {
        Ok(())
    }
}
