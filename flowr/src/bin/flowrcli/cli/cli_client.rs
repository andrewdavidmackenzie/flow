use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::sync::{Arc, Mutex};

use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
use log::{debug, error, info};
use tokio::sync::mpsc;

use flowcore::errors::Result;

use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};
use flowrlib::connections::ClientConnection;

const DEFAULT_NAME: &str = "unknown";

/// Request sent to the background stdin reader thread.
enum StdinRequest {
    ReadLine(String),
    ReadAll,
}

#[derive(Debug, Clone)]
pub struct CliRuntimeClient {
    args: Vec<String>,
    override_args: Arc<Mutex<Vec<String>>>,
    image_buffers: HashMap<String, ImageBuffer<Rgb<u8>, Vec<u8>>>,
    #[cfg(feature = "metrics")]
    display_metrics: bool,
}

impl CliRuntimeClient {
    /// Create a new runtime client
    pub fn new(
        args: Vec<String>,
        override_args: Arc<Mutex<Vec<String>>>,
        #[cfg(feature = "metrics")] display_metrics: bool,
    ) -> Self {
        CliRuntimeClient {
            args,
            override_args,
            image_buffers: HashMap::<String, ImageBuffer<Rgb<u8>, Vec<u8>>>::new(),
            #[cfg(feature = "metrics")]
            display_metrics,
        }
    }

    /// Enter an async loop where we receive events as a client and respond to them.
    ///
    /// Stdin is read on a separate background thread so the event loop can
    /// continue processing stdout and other messages while waiting for user input.
    pub async fn event_loop(
        mut self,
        connection: ClientConnection,
        blocking_io_connection: ClientConnection,
    ) -> Result<()> {
        let (event_tx, mut event_rx) = mpsc::channel::<CoordinatorMessage>(32);
        let (response_tx, mut response_rx) = mpsc::channel::<ClientMessage>(32);

        let bridge =
            tokio::task::spawn_blocking(move || zmq_bridge(connection, event_tx, &mut response_rx));

        // Blocking IO bridge: separate ZMQ bridge for GetLine/GetStdin
        let (blocking_event_tx, mut blocking_event_rx) = mpsc::channel::<CoordinatorMessage>(1);
        let (blocking_response_tx, mut blocking_response_rx) = mpsc::channel::<ClientMessage>(1);

        // Set a receive timeout so the blocking IO bridge can detect when
        // the coordinator shuts down and exit cleanly
        let _ = blocking_io_connection.set_receive_timeout(2000);
        let blocking_bridge = tokio::task::spawn_blocking(move || {
            blocking_io_zmq_bridge(
                blocking_io_connection,
                blocking_event_tx,
                &mut blocking_response_rx,
            );
        });

        let (stdin_req_tx, mut stdin_req_rx) = mpsc::channel::<StdinRequest>(1);
        let (stdin_resp_tx, mut stdin_resp_rx) = mpsc::channel::<ClientMessage>(1);

        let stdin_thread = tokio::task::spawn_blocking(move || {
            stdin_reader(&mut stdin_req_rx, &stdin_resp_tx);
        });

        let result = loop {
            tokio::select! {
                event = event_rx.recv() => {
                    match event {
                        Some(event) => {
                            let response = self.process_coordinator_message(event);
                            if let ClientMessage::ClientExiting(ref coordinator_result) = response {
                                debug!("Client is exiting the event loop.");
                                let exit_result = coordinator_result.clone();
                                if let Err(e) = response_tx.send(response).await {
                                    error!("Failed to send ClientExiting to bridge: {e}");
                                }
                                break exit_result;
                            }
                            if let Err(e) = response_tx.send(response).await {
                                error!("Failed to send response to bridge: {e}");
                                break Err("Bridge channel closed".into());
                            }
                        }
                        None => break Err("ZMQ bridge closed unexpectedly".into()),
                    }
                }
                blocking_event = blocking_event_rx.recv() => {
                    match blocking_event {
                        Some(CoordinatorMessage::GetLine(prompt)) => {
                            if stdin_req_tx.send(StdinRequest::ReadLine(prompt)).await.is_err() {
                                let _ = blocking_response_tx.send(
                                    ClientMessage::Error("Stdin reader closed".into())
                                ).await;
                            }
                        }
                        Some(CoordinatorMessage::GetStdin) => {
                            if stdin_req_tx.send(StdinRequest::ReadAll).await.is_err() {
                                let _ = blocking_response_tx.send(
                                    ClientMessage::Error("Stdin reader closed".into())
                                ).await;
                            }
                        }
                        Some(other) => {
                            debug!("Unexpected message on blocking IO bridge: {other}");
                            let _ = blocking_response_tx.send(ClientMessage::Ack).await;
                        }
                        None => {
                            debug!("Blocking IO bridge closed");
                        }
                    }
                }
                stdin_response = stdin_resp_rx.recv() => {
                    if let Some(response) = stdin_response {
                        if let Err(e) = blocking_response_tx.send(response).await {
                            error!("Failed to send stdin response to blocking bridge: {e}");
                            break Err("Blocking bridge channel closed".into());
                        }
                    }
                }
            }
        };

        drop(response_tx);
        drop(blocking_response_tx);
        drop(stdin_req_tx);
        let _ = bridge.await;
        let _ = blocking_bridge.await;
        let _ = stdin_thread.await;

        result
    }

    /// Event loop driven by channels directly — used for testing without ZMQ.
    #[cfg(test)]
    pub async fn event_loop_on_channels(
        mut self,
        mut event_rx: mpsc::Receiver<CoordinatorMessage>,
        response_tx: mpsc::Sender<ClientMessage>,
    ) -> Result<()> {
        loop {
            match event_rx.recv().await {
                Some(event) => {
                    let response = self.process_coordinator_message(event);
                    if let ClientMessage::ClientExiting(ref coordinator_result) = response {
                        debug!("Client is exiting the event loop.");
                        let exit_result = coordinator_result.clone();
                        if let Err(e) = response_tx.send(response).await {
                            error!("Failed to send ClientExiting: {e}");
                        }
                        return exit_result;
                    }
                    if let Err(e) = response_tx.send(response).await {
                        error!("Failed to send response: {e}");
                        return Err("Channel closed".into());
                    }
                }
                None => return Err("Event channel closed".into()),
            }
        }
    }

    fn flush_image_buffers(&mut self) {
        for (filename, image_buffer) in self.image_buffers.drain() {
            info!("Flushing ImageBuffer to file: {filename}");
            if let Err(e) = image_buffer.save_with_format(Path::new(&filename), ImageFormat::Png) {
                error!("Error saving ImageBuffer '{filename}': '{e}'");
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    #[allow(clippy::many_single_char_names)]
    fn process_coordinator_message(&mut self, message: CoordinatorMessage) -> ClientMessage {
        match message {
            #[cfg(feature = "metrics")]
            CoordinatorMessage::FlowEnd(metrics) => {
                debug!("=========================== Flow execution ended ======================================");
                if self.display_metrics {
                    println!("\nMetrics: \n{metrics}");
                    let _ = io::stdout().flush();
                }
                self.flush_image_buffers();
                ClientMessage::ClientExiting(Ok(()))
            }
            #[cfg(not(feature = "metrics"))]
            CoordinatorMessage::FlowEnd => {
                debug!("=========================== Flow execution ended ======================================");
                self.flush_image_buffers();
                ClientMessage::ClientExiting(Ok(()))
            }
            CoordinatorMessage::FlowStart => {
                debug!("===========================    Starting flow execution =============================");
                ClientMessage::Ack
            }
            CoordinatorMessage::CoordinatorExiting(result) => {
                debug!("Coordinator is exiting");
                ClientMessage::ClientExiting(result)
            }
            CoordinatorMessage::StdoutEof => ClientMessage::Ack,
            CoordinatorMessage::Stdout(contents) => {
                let stdout = io::stdout();
                let mut handle = stdout.lock();
                let _ = handle.write_all(format!("{contents}\n").as_bytes());
                let _ = io::stdout().flush();
                ClientMessage::Ack
            }
            CoordinatorMessage::StderrEof => ClientMessage::Ack,
            CoordinatorMessage::Stderr(contents) => {
                let stderr = io::stderr();
                let mut handle = stderr.lock();
                let _ = handle.write_all(format!("{contents}\n").as_bytes());
                let _ = io::stdout().flush();
                ClientMessage::Ack
            }
            CoordinatorMessage::GetStdin | CoordinatorMessage::GetLine(_) => {
                unreachable!("Handled in event_loop before calling process_coordinator_message")
            }
            CoordinatorMessage::Read(file_path) => match File::open(&file_path) {
                Ok(mut f) => {
                    let mut buffer = Vec::new();
                    match f.read_to_end(&mut buffer) {
                        Ok(_) => ClientMessage::FileContents(file_path, buffer),
                        Err(_) => ClientMessage::Error(format!(
                            "Could not read content from '{file_path:?}'"
                        )),
                    }
                }
                Err(_) => ClientMessage::Error(format!("Could not open file '{file_path:?}'")),
            },
            CoordinatorMessage::Write(filename, bytes) => match File::create(&filename) {
                Ok(mut file) => match file.write_all(bytes.as_slice()) {
                    Ok(()) => ClientMessage::Ack,
                    Err(e) => {
                        let msg = format!("Error writing to file: '{filename}': '{e}'");
                        error!("{msg}");
                        ClientMessage::Error(msg)
                    }
                },
                Err(e) => {
                    let msg = format!("Error creating file: '{filename}': '{e}'");
                    error!("{msg}");
                    ClientMessage::Error(msg)
                }
            },
            #[allow(clippy::many_single_char_names)]
            CoordinatorMessage::PixelWrite((x, y), (r, g, b), (width, height), name) => {
                let image = self
                    .image_buffers
                    .entry(name)
                    .or_insert_with(|| RgbImage::new(width, height));
                image.put_pixel(x, y, Rgb([r, g, b]));
                ClientMessage::Ack
            }
            CoordinatorMessage::ImageWrite(grid, name) => {
                let height = u32::try_from(grid.len()).unwrap_or(0);
                let width = grid
                    .first()
                    .map_or(0, |row| u32::try_from(row.len()).unwrap_or(0));
                let image = self
                    .image_buffers
                    .entry(name)
                    .or_insert_with(|| RgbImage::new(width, height));
                for (y, row) in grid.iter().enumerate() {
                    for (x, &val) in row.iter().enumerate() {
                        let gray = val;
                        image.put_pixel(
                            u32::try_from(x).unwrap_or(0),
                            u32::try_from(y).unwrap_or(0),
                            Rgb([gray, gray, gray]),
                        );
                    }
                }
                ClientMessage::Ack
            }
            CoordinatorMessage::GetArgs => {
                if let Ok(override_args) = self.override_args.lock() {
                    if override_args.is_empty() {
                        ClientMessage::Args(self.args.clone())
                    } else {
                        let arg_zero = self
                            .args
                            .first()
                            .unwrap_or(&DEFAULT_NAME.to_string())
                            .to_owned();
                        let mut one_time_args = vec![arg_zero];
                        one_time_args.append(&mut override_args.to_vec());
                        ClientMessage::Args(one_time_args)
                    }
                } else {
                    ClientMessage::Args(self.args.clone())
                }
            }
            CoordinatorMessage::Invalid => ClientMessage::Ack,
        }
    }
}

/// Background thread that reads from stdin when requested.
fn stdin_reader(req_rx: &mut mpsc::Receiver<StdinRequest>, resp_tx: &mpsc::Sender<ClientMessage>) {
    while let Some(request) = req_rx.blocking_recv() {
        let response = match request {
            StdinRequest::ReadLine(_prompt) => {
                let mut input = String::new();
                match io::stdin().lock().read_line(&mut input) {
                    Ok(n) if n > 0 => ClientMessage::Line(input.trim().to_string()),
                    Ok(0) => ClientMessage::GetLineEof,
                    _ => ClientMessage::Error("Could not read Readline".into()),
                }
            }
            StdinRequest::ReadAll => {
                let mut buffer = String::new();
                match io::stdin().read_to_string(&mut buffer) {
                    Ok(size) if size > 0 => ClientMessage::Stdin(buffer.trim().to_string()),
                    Ok(_) => ClientMessage::GetStdinEof,
                    Err(_) => ClientMessage::Error("Could not read Stdin".into()),
                }
            }
        };
        if resp_tx.blocking_send(response).is_err() {
            break;
        }
    }
}

/// Client-side ZMQ bridge for blocking IO (readline/stdin) on a separate socket.
///
/// Protocol (2 REQ/REP round-trips per blocking IO operation):
/// 1. Send `Ack` to coordinator (initiate polling)
/// 2. Receive `GetLine`/`GetStdin` from coordinator
/// 3. Forward to event loop, wait for stdin result
/// 4. Send `Line`/`Stdin` result to coordinator
/// 5. Receive `Invalid` acknowledgement from coordinator
/// 6. Loop
#[allow(clippy::needless_pass_by_value)]
fn blocking_io_zmq_bridge(
    connection: ClientConnection,
    event_tx: mpsc::Sender<CoordinatorMessage>,
    response_rx: &mut mpsc::Receiver<ClientMessage>,
) {
    loop {
        // Send Ack to poll the coordinator for a blocking IO request
        if let Err(e) = connection.send(ClientMessage::Ack) {
            debug!("Blocking IO bridge: send failed (shutdown expected): {e}");
            break;
        }

        // Receive the blocking IO request (GetLine/GetStdin)
        match connection.receive::<CoordinatorMessage>() {
            Ok(event) => {
                // Forward to event loop
                if event_tx.blocking_send(event).is_err() {
                    break;
                }

                // Wait for the response from the event loop (stdin reader)
                match response_rx.blocking_recv() {
                    Some(response) => {
                        // Send the result back to the coordinator
                        if let Err(e) = connection.send(response) {
                            debug!("Blocking IO bridge: send failed: {e}");
                            break;
                        }

                        // Receive the coordinator's acknowledgement
                        match connection.receive::<CoordinatorMessage>() {
                            Ok(_) => {} // Expected: Invalid or similar ack
                            Err(e) => {
                                debug!("Blocking IO bridge: recv ack failed: {e}");
                                break;
                            }
                        }
                    }
                    None => break,
                }
            }
            Err(_) => {
                // Expected during shutdown — receive timeout or coordinator gone
                break;
            }
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn zmq_bridge(
    connection: ClientConnection,
    event_tx: mpsc::Sender<CoordinatorMessage>,
    response_rx: &mut mpsc::Receiver<ClientMessage>,
) {
    loop {
        match connection.receive::<CoordinatorMessage>() {
            Ok(event) => {
                if event_tx.blocking_send(event).is_err() {
                    break;
                }
                match response_rx.blocking_recv() {
                    Some(response) => {
                        if let Err(e) = connection.send(response) {
                            error!("ZMQ bridge: failed to send response: {e}");
                            break;
                        }
                    }
                    None => break,
                }
            }
            Err(e) => {
                error!("ZMQ bridge: failed to receive: {e}");
                break;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use std::fs;
    use std::fs::File;
    use std::io::prelude::*;
    use std::sync::{Arc, Mutex};

    use tempfile::tempdir;

    #[cfg(feature = "metrics")]
    use flowcore::model::metrics::Metrics;

    use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};

    use super::CliRuntimeClient;

    fn make_client() -> CliRuntimeClient {
        CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string(), "1".to_string()],
            Arc::new(Mutex::new(vec![])),
            #[cfg(feature = "metrics")]
            false,
        )
    }

    #[test]
    fn test_arg_passing() {
        let mut client = make_client();
        match client.process_coordinator_message(CoordinatorMessage::GetArgs) {
            ClientMessage::Args(args) => assert_eq!(
                vec!("file:///test_flow.toml".to_string(), "1".to_string()),
                args
            ),
            _ => panic!("Didn't get Args response as expected"),
        }
    }

    #[test]
    fn test_arg_overriding() {
        let override_args = Arc::new(Mutex::new(vec![]));
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string(), "1".to_string()],
            override_args.clone(),
            #[cfg(feature = "metrics")]
            false,
        );
        {
            let mut overrides = override_args.lock().expect("Could not lock override args");
            overrides.push("override".into());
        }
        match client.process_coordinator_message(CoordinatorMessage::GetArgs) {
            ClientMessage::Args(args) => assert_eq!(
                vec!("file:///test_flow.toml".to_string(), "override".to_string()),
                args
            ),
            _ => panic!("Args override response was not as expected"),
        }
    }

    #[test]
    fn test_file_reading() {
        let test_contents = b"The quick brown fox jumped over the lazy dog";
        let temp = tempdir().expect("Couldn't get temporary directory").keep();
        let file_path = temp.join("test_read").to_string_lossy().to_string();
        {
            let mut file = File::create(&file_path).expect("Could not create test file");
            file.write_all(test_contents).expect("Could not write");
        }
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string()],
            Arc::new(Mutex::new(vec![])),
            #[cfg(feature = "metrics")]
            false,
        );
        match client.process_coordinator_message(CoordinatorMessage::Read(file_path.clone())) {
            ClientMessage::FileContents(path_read, contents) => {
                assert_eq!(path_read, file_path);
                assert_eq!(contents, test_contents);
            }
            _ => panic!("Didn't get Read response as expected"),
        }
    }

    #[test]
    fn test_file_writing() {
        let temp = tempdir().expect("Couldn't get temporary directory").keep();
        let file = temp.join("test");
        let mut client = make_client();
        match client.process_coordinator_message(CoordinatorMessage::Write(
            file.to_str().expect("Couldn't get filename").to_string(),
            b"Hello".to_vec(),
        )) {
            ClientMessage::Ack => {}
            _ => panic!("Didn't get Write response as expected"),
        }
    }

    #[test]
    fn test_stdout() {
        let mut client = make_client();
        match client.process_coordinator_message(CoordinatorMessage::Stdout("Hello".into())) {
            ClientMessage::Ack => {}
            _ => panic!("Didn't get Stdout response as expected"),
        }
    }

    #[test]
    fn test_stderr() {
        let mut client = make_client();
        match client.process_coordinator_message(CoordinatorMessage::Stderr("Hello".into())) {
            ClientMessage::Ack => {}
            _ => panic!("Didn't get Stderr response as expected"),
        }
    }

    #[test]
    fn test_image_writing() {
        let mut client = make_client();
        let temp_dir = tempdir().expect("Couldn't get temporary directory").keep();
        let path = temp_dir.join("flow.png");
        let _ = fs::remove_file(&path);
        assert!(!path.exists());

        client.process_coordinator_message(CoordinatorMessage::FlowStart);
        match client.process_coordinator_message(CoordinatorMessage::PixelWrite(
            (0, 0),
            (255, 200, 20),
            (10, 10),
            path.display().to_string(),
        )) {
            ClientMessage::Ack => {}
            _ => panic!("Didn't get pixel write response as expected"),
        }

        #[cfg(not(feature = "metrics"))]
        client.process_coordinator_message(CoordinatorMessage::FlowEnd);
        #[cfg(feature = "metrics")]
        client.process_coordinator_message(CoordinatorMessage::FlowEnd(Metrics::new(1, 1)));

        assert!(path.exists(), "Image file was not created");
    }

    #[test]
    fn coordinator_exiting() {
        let mut client = make_client();
        match client.process_coordinator_message(CoordinatorMessage::CoordinatorExiting(Ok(()))) {
            ClientMessage::ClientExiting(_) => {}
            _ => panic!("Didn't get ClientExiting response as expected"),
        }
    }

    #[tokio::test]
    async fn async_event_loop_processes_messages() {
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(10);
        let (response_tx, mut response_rx) = tokio::sync::mpsc::channel(10);

        let client = CliRuntimeClient::new(
            vec!["file:///test.toml".to_string()],
            Arc::new(Mutex::new(vec![])),
            #[cfg(feature = "metrics")]
            false,
        );

        let handle =
            tokio::spawn(async move { client.event_loop_on_channels(event_rx, response_tx).await });

        event_tx.send(CoordinatorMessage::FlowStart).await.unwrap();
        assert!(matches!(
            response_rx.recv().await.unwrap(),
            ClientMessage::Ack
        ));

        event_tx
            .send(CoordinatorMessage::Stdout("hello".into()))
            .await
            .unwrap();
        assert!(matches!(
            response_rx.recv().await.unwrap(),
            ClientMessage::Ack
        ));

        #[cfg(not(feature = "metrics"))]
        event_tx.send(CoordinatorMessage::FlowEnd).await.unwrap();
        #[cfg(feature = "metrics")]
        event_tx
            .send(CoordinatorMessage::FlowEnd(Metrics::new(1, 1)))
            .await
            .unwrap();

        assert!(matches!(
            response_rx.recv().await.unwrap(),
            ClientMessage::ClientExiting(Ok(()))
        ));

        assert!(handle.await.unwrap().is_ok());
    }
}
