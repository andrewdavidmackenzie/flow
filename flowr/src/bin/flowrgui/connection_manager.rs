use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use iced::futures::SinkExt;
use iced::Subscription;
use log::{error, info, trace};
use portpicker::pick_unused_port;
use tokio::sync::mpsc::Receiver;
use url::Url;

use flowcore::meta_provider::MetaProvider;
use flowcore::provider::Provider;
use flowrlib::coordinator::Coordinator;
use flowrlib::dispatcher::Dispatcher;
use flowrlib::executor::Executor;
use flowrlib::services::{CONTROL_SERVICE_NAME, JOB_SERVICE_NAME, RESULTS_JOB_SERVICE_NAME};

use crate::errors::{Result, ResultExt};
use crate::gui::client_message::ClientMessage;
use crate::gui::coordinator_message::CoordinatorMessage;
#[cfg(feature = "submission")]
use crate::gui::submission_handler::CLISubmissionHandler;
use crate::{context, CoordinatorSettings, Message, ServerSettings};
use flowrlib::connections::ClientConnection;
use flowrlib::connections::CoordinatorConnection;
#[cfg(feature = "debugger")]
use flowrlib::debug_zmq_handler::DebugZmqHandler;
use flowrlib::discovery::discover_service;
use flowrlib::discovery::enable_service_discovery;
use flowrlib::services::COORDINATOR_SERVICE_NAME;
#[cfg(feature = "debugger")]
use flowrlib::services::DEBUG_SERVICE_NAME;

#[cfg(feature = "debugger")]
use flowcore::model::debug_command::DebugCommand;
#[cfg(feature = "debugger")]
use flowrlib::debug_server_message::DebugServerMessage;

/// States in which the Connection to the Coordinator can find itself
pub enum CoordinatorState {
    Init(ServerSettings),
    Discovery,
    Discovered(String),
    Connected(Receiver<ClientMessage>, Arc<Mutex<ClientConnection>>),
}

/// Global storage for coordinator settings, initialized once and read by the subscription
static COORDINATOR_SETTINGS: OnceLock<CoordinatorSettings> = OnceLock::new();

/// Global flag for requesting flow stop from the GUI
static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Request the currently running flow to stop
pub fn request_stop() {
    STOP_REQUESTED.store(true, Ordering::Relaxed);
}

/// Check and clear the stop request flag
pub fn take_stop_request() -> bool {
    STOP_REQUESTED.swap(false, Ordering::Relaxed)
}

/// Global debug port, set when the debug server starts
#[cfg(feature = "debugger")]
static DEBUG_PORT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(0);

/// Get the debug port (0 means not set)
#[cfg(feature = "debugger")]
pub fn get_debug_port() -> u16 {
    DEBUG_PORT.load(Ordering::Relaxed)
}

/// Global counter for jobs created, updated by the coordinator thread
static JOB_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Set the current job count (called from coordinator thread)
pub fn set_job_count(count: usize) {
    JOB_COUNT.store(count, Ordering::Relaxed);
}

/// Get the current job count (called from GUI thread)
pub fn get_job_count() -> usize {
    JOB_COUNT.load(Ordering::Relaxed)
}

// Creates an asynchronous worker that sends messages back and forth between the App and
// the Coordinator
pub fn subscribe(coordinator_settings: CoordinatorSettings) -> Subscription<CoordinatorMessage> {
    COORDINATOR_SETTINGS.get_or_init(|| coordinator_settings);
    Subscription::run(coordinator_stream)
}

fn coordinator_stream() -> impl iced::futures::Stream<Item = CoordinatorMessage> {
    let Some(settings) = COORDINATOR_SETTINGS.get().cloned() else {
        eprintln!("Error: Coordinator settings were not initialized before subscribing. This is a programming error.");
        std::process::exit(1);
    };
    iced::stream::channel(
        100,
        move |mut app_sender: iced::futures::channel::mpsc::Sender<CoordinatorMessage>| async move {
            let mut state = match settings {
                CoordinatorSettings::Server(sett) => CoordinatorState::Init(sett.clone()),
                CoordinatorSettings::ClientOnly(_) => CoordinatorState::Discovery,
            };

            let mut running = false;
            loop {
                match state {
                    CoordinatorState::Init(settings) => match start_server(settings) {
                        Ok(()) => state = CoordinatorState::Discovery,
                        Err(e) => {
                            let _ = app_sender
                                .send(CoordinatorMessage::Disconnected(format!(
                                    "Could not start coordinator server: {e}"
                                )))
                                .await;
                            break;
                        }
                    },

                    CoordinatorState::Discovery => {
                        match discover_service(COORDINATOR_SERVICE_NAME) {
                            Ok(address) => state = CoordinatorState::Discovered(address),
                            Err(e) => {
                                let _ = app_sender
                                    .send(CoordinatorMessage::Disconnected(format!(
                                        "Could not discover coordinator: {e}"
                                    )))
                                    .await;
                                break;
                            }
                        }
                    }

                    CoordinatorState::Discovered(address) => {
                        match ClientConnection::new(&address) {
                            Ok(connection) => {
                                let (app_side_sender, app_receiver) =
                                    tokio::sync::mpsc::channel(100);

                                if app_sender
                                    .send(CoordinatorMessage::Connected(app_side_sender))
                                    .await
                                    .is_err()
                                {
                                    error!("Could not send Connected message to app");
                                    break;
                                }

                                state = CoordinatorState::Connected(
                                    app_receiver,
                                    Arc::new(Mutex::new(connection)),
                                );
                            }
                            Err(e) => {
                                let _ = app_sender
                                    .send(CoordinatorMessage::Disconnected(format!(
                                        "Could not connect to coordinator: {e}"
                                    )))
                                    .await;
                                break;
                            }
                        }
                    }

                    CoordinatorState::Connected(ref mut app_receiver, ref connection) => {
                        let result = if running {
                            handle_running(connection, &mut app_sender, app_receiver, &mut running)
                                .await
                        } else {
                            handle_idle(connection, &mut app_sender, app_receiver, &mut running)
                                .await
                        };
                        if result.is_err() {
                            break;
                        }
                    }
                }
            }
        },
    )
}

fn lock_and_receive(
    connection: &Arc<Mutex<ClientConnection>>,
) -> std::result::Result<CoordinatorMessage, String> {
    connection
        .lock()
        .map_err(|e| format!("Lock error: {e}"))
        .and_then(|conn| conn.receive().map_err(|e| format!("{e}")))
}

fn lock_and_send(
    connection: &Arc<Mutex<ClientConnection>>,
    msg: ClientMessage,
) -> std::result::Result<(), String> {
    connection
        .lock()
        .map_err(|e| format!("Lock error: {e}"))
        .and_then(|conn| conn.send(msg).map_err(|e| format!("{e}")))
}

async fn handle_running(
    connection: &Arc<Mutex<ClientConnection>>,
    app_sender: &mut iced::futures::channel::mpsc::Sender<CoordinatorMessage>,
    app_receiver: &mut Receiver<ClientMessage>,
    running: &mut bool,
) -> std::result::Result<(), ()> {
    let coordinator_message = lock_and_receive(connection).map_err(|e| {
        let _ = app_sender.try_send(CoordinatorMessage::Disconnected(format!(
            "Lost connection to coordinator: {e}"
        )));
    })?;

    let is_flow_end = matches!(&coordinator_message, &CoordinatorMessage::FlowEnd(_));

    if app_sender.send(coordinator_message).await.is_err() {
        error!("Could not forward coordinator message to app");
        return Err(());
    }

    if is_flow_end {
        *running = false;
        return Ok(());
    }

    if let Some(client_message) = app_receiver.recv().await {
        lock_and_send(connection, client_message).map_err(|e| {
            let _ = app_sender.try_send(CoordinatorMessage::Disconnected(format!(
                "Lost connection to coordinator: {e}"
            )));
        })
    } else {
        error!("App channel closed while running");
        Err(())
    }
}

async fn handle_idle(
    connection: &Arc<Mutex<ClientConnection>>,
    app_sender: &mut iced::futures::channel::mpsc::Sender<CoordinatorMessage>,
    app_receiver: &mut Receiver<ClientMessage>,
    running: &mut bool,
) -> std::result::Result<(), ()> {
    if let Some(client_message) = app_receiver.recv().await {
        lock_and_send(connection, client_message).map_err(|e| {
            let _ = app_sender.try_send(CoordinatorMessage::Disconnected(format!(
                "Could not submit flow: {e}"
            )));
        })?;
        *running = true;
        Ok(())
    } else {
        error!("App channel closed");
        Err(())
    }
}

// Start a coordinator server in a background thread, then discover it and return the address
fn start_server(coordinator_settings: ServerSettings) -> Result<()> {
    let runtime_port = pick_unused_port().chain_err(|| "No ports free")?;
    let coordinator_connection =
        CoordinatorConnection::new(COORDINATOR_SERVICE_NAME, runtime_port)?;

    let _mdns_coordinator = enable_service_discovery(COORDINATOR_SERVICE_NAME, runtime_port)?;

    #[cfg(feature = "debugger")]
    let debug_port = pick_unused_port().chain_err(|| "No ports free")?;
    #[cfg(feature = "debugger")]
    let debug_connection = CoordinatorConnection::new(DEBUG_SERVICE_NAME, debug_port)?;
    #[cfg(feature = "debugger")]
    let _mdns_debug = enable_service_discovery(DEBUG_SERVICE_NAME, debug_port)?;

    #[cfg(feature = "debugger")]
    {
        DEBUG_PORT.store(debug_port, Ordering::Relaxed);
        info!("Debug server listening on port {debug_port}. Connect with: flowrdb --address localhost:{debug_port}");
    }

    info!("Starting coordinator in background thread");
    thread::spawn(move || {
        if let Err(e) = coordinator(
            coordinator_settings,
            coordinator_connection,
            #[cfg(feature = "debugger")]
            debug_connection,
            true,
        ) {
            error!("Coordinator thread exited with error: {e}");
        }
    });

    Ok(())
}

fn coordinator(
    coordinator_settings: ServerSettings,
    coordinator_connection: CoordinatorConnection,
    #[cfg(feature = "debugger")] debug_connection: CoordinatorConnection,
    loop_forever: bool,
) -> Result<()> {
    let connection = Arc::new(Mutex::new(coordinator_connection));

    #[cfg(feature = "debugger")]
    let mut debug_server = DebugZmqHandler {
        debug_server_connection: debug_connection,
    };

    let provider = Arc::new(MetaProvider::new(
        coordinator_settings.lib_search_path,
        PathBuf::from("/"),
    )) as Arc<dyn Provider>;

    let ports = get_four_ports()?;
    trace!("Announcing three job queues and a control socket on ports: {ports:?}");
    let job_queues = get_bind_addresses(ports);
    let dispatcher = Dispatcher::new(&job_queues)?;
    let _mdns_jobs = enable_service_discovery(JOB_SERVICE_NAME, ports.0)?;
    let _mdns_results = enable_service_discovery(RESULTS_JOB_SERVICE_NAME, ports.2)?;
    let _mdns_control = enable_service_discovery(CONTROL_SERVICE_NAME, ports.3)?;

    let (job_source_name, context_job_source_name, results_sink, control_socket) =
        get_connect_addresses(ports);

    let mut executor = Executor::new();
    // if the command line options request loading native implementation of available native libs
    // if not, the native implementation is not loaded and later when a flow is loaded it's library
    // references will be resolved and those libraries (WASM implementations) will be loaded at runtime
    #[cfg(feature = "flowstdlib")]
    if coordinator_settings.native_flowstdlib {
        executor.add_lib(
            flowstdlib::manifest::get()
                .chain_err(|| "Could not get 'native' flowstdlib manifest")?,
            Url::parse("memory://")?, // Statically linked library has no resolved Url
        )?;
    }
    executor.start(
        &provider,
        coordinator_settings.num_threads,
        &job_source_name,
        &results_sink,
        &control_socket,
    );

    let mut context_executor = Executor::new();
    context_executor.add_lib(
        context::get_manifest(connection.clone())?,
        Url::parse("memory://")?, // Statically linked library has no resolved Url
    )?;
    context_executor.start(
        &provider,
        1,
        &context_job_source_name,
        &results_sink,
        &control_socket,
    );

    #[cfg(feature = "submission")]
    let mut submitter = CLISubmissionHandler::new(connection);

    let mut coordinator = Coordinator::new(
        dispatcher,
        #[cfg(feature = "submission")]
        &mut submitter,
        #[cfg(feature = "debugger")]
        &mut debug_server,
    );

    #[cfg(feature = "submission")]
    {
        coordinator.submission_loop(loop_forever)?;
    }
    Ok(())
}

/// Global sender for debug commands from GUI to the debug client subscription
#[cfg(feature = "debugger")]
static DEBUG_CMD_SENDER: std::sync::RwLock<Option<tokio::sync::mpsc::Sender<DebugCommand>>> =
    std::sync::RwLock::new(None);

/// Send a debug command from the GUI to the debug client subscription
#[cfg(feature = "debugger")]
pub fn send_debug_command(cmd: DebugCommand) {
    if let Ok(guard) = DEBUG_CMD_SENDER.read() {
        if let Some(ref sender) = *guard {
            let _ = sender.blocking_send(cmd);
        }
    }
}

/// Rewrite server messages that contain CLI-oriented text for the GUI context
#[cfg(feature = "debugger")]
fn rewrite_for_gui(msg: &str) -> String {
    msg.replace(
        "Use the 'b' command to set a breakpoint. Use 'h' for help.",
        "Use the 'Set BP' button to set a breakpoint.",
    )
    .replace(
        "Use 'i n' or 'inspect n' to inspect the function number 'n'",
        "Enter a function number in the spec field and click 'Inspect'",
    )
    .replace(
        "State variables that can be modified are:",
        "Modifiable state variables:",
    )
}

/// Format a `DebugServerMessage` into a human-readable string for the debug tab
#[cfg(feature = "debugger")]
fn format_debug_event(message: &DebugServerMessage) -> String {
    match message {
        DebugServerMessage::JobCompleted(job) => {
            let mut s = format!(
                "Job #{} completed by Function #{}",
                job.payload.job_id, job.process_id
            );
            if let Ok((Some(output), _)) = &job.result {
                use std::fmt::Write;
                let _ = write!(s, "\n\tOutput value: '{output}'");
            }
            s
        }
        DebugServerMessage::PriorToSendingJob(job) => {
            format!(
                "About to send Job #{} to Function #{}\n\tInputs: {:?}",
                job.payload.job_id, job.process_id, job.payload.input_set
            )
        }
        DebugServerMessage::BlockBreakpoint(block) => format!("Block breakpoint: {block:?}"),
        DebugServerMessage::DataBreakpoint(
            source_name,
            source_id,
            output_route,
            value,
            dest_id,
            dest_name,
            io_name,
            input_number,
        ) => {
            format!(
                "Data breakpoint: Function #{source_id} '{source_name}{output_route}' \
                --{value}-> Function #{dest_id}:{input_number} '{dest_name}'/'{io_name}'"
            )
        }
        DebugServerMessage::Panic(message, jobs_created) => {
            format!("Function panicked after {jobs_created} jobs created: {message}")
        }
        DebugServerMessage::JobError(job) => format!("Error executing Job: '{job}'"),
        DebugServerMessage::Deadlock(message) => format!("Deadlock detected: {message}"),
        DebugServerMessage::EnteringDebugger => {
            "Entering debugger. Use the controls above to debug.".into()
        }
        DebugServerMessage::ExitingDebugger => "Debugger is exiting".into(),
        DebugServerMessage::ExecutionStarted => "Running flow".into(),
        DebugServerMessage::ExecutionEnded => "Flow has completed".into(),
        DebugServerMessage::Functions(functions) => {
            use std::fmt::Write;
            let mut s = String::from("Functions List\n");
            for f in functions {
                let _ = writeln!(s, "\t#{} '{}' @ '{}'", f.id(), f.name(), f.route());
            }
            s
        }
        DebugServerMessage::SendingValue(source_id, value, dest_id, input_number) => {
            format!("Function #{source_id} sending '{value}' to {dest_id}:{input_number}")
        }
        DebugServerMessage::Error(msg) => msg.clone(),
        DebugServerMessage::Message(msg) => rewrite_for_gui(msg),
        DebugServerMessage::Resetting => "Resetting state".into(),
        DebugServerMessage::FunctionStates((function, states)) => {
            format!("{function}\n\tState: {states:?}")
        }
        DebugServerMessage::OverallState(run_state) => format!("{run_state}"),
        DebugServerMessage::InputState(input) => format!("{input}"),
        DebugServerMessage::OutputState(connections) => {
            if connections.is_empty() {
                "No output connections from that sub-route".into()
            } else {
                connections
                    .iter()
                    .map(|c| format!("{c}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        DebugServerMessage::BlockState(blocks) => {
            if blocks.is_empty() {
                "No blocks matching the specification were found".into()
            } else {
                blocks
                    .iter()
                    .map(|b| format!("{b}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        DebugServerMessage::FlowUnblockBreakpoint(flow_id) => {
            format!("Flow #{flow_id} was busy and has now gone idle, unblocking senders")
        }
        DebugServerMessage::WaitingForCommand(job_id) => {
            format!("Waiting for command (Job #{job_id})")
        }
        DebugServerMessage::Invalid => "Invalid message from debug server".into(),
    }
}

/// Create a subscription that runs a debug client, connecting to the debug server via ZMQ
#[cfg(feature = "debugger")]
pub fn debug_client_subscribe(port: u16) -> Subscription<Message> {
    Subscription::run_with(port, |port| debug_client_stream(*port))
}

#[cfg(feature = "debugger")]
fn debug_client_stream(port: u16) -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(
        100,
        move |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
            let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::channel::<DebugCommand>(10);
            if let Ok(mut guard) = DEBUG_CMD_SENDER.write() {
                *guard = Some(cmd_tx);
            }

            let mut blocking_sender = sender.clone();
            let result = tokio::task::spawn_blocking(move || -> std::result::Result<(), String> {
                let address = format!("localhost:{port}");
                let connection = ClientConnection::new(&address)
                    .map_err(|e| format!("Could not connect to debug server: {e}"))?;
                if let Err(e) = connection.set_receive_timeout(5_000) {
                    error!("Could not set receive timeout: {e}");
                }

                connection
                    .send(DebugCommand::DebugClientStarting)
                    .map_err(|e| format!("Could not send starting command: {e}"))?;

                let _ = blocking_sender.try_send(Message::DebugConnected);

                loop {
                    let message: DebugServerMessage = match connection.receive() {
                        Ok(msg) => msg,
                        Err(e) => {
                            let _ = blocking_sender
                                .try_send(Message::DebugDisconnected(format!("{e}")));
                            break;
                        }
                    };

                    let is_exiting = matches!(message, DebugServerMessage::ExitingDebugger);
                    let is_waiting = matches!(message, DebugServerMessage::WaitingForCommand(_));

                    if !is_waiting {
                        let _ = blocking_sender
                            .try_send(Message::DebugEvent(format_debug_event(&message)));
                    }

                    if is_exiting {
                        let _ = connection.send(DebugCommand::Ack);
                        let _ = blocking_sender
                            .try_send(Message::DebugDisconnected("Debugger exited".into()));
                        break;
                    }

                    if is_waiting {
                        let _ = blocking_sender.try_send(Message::DebugWaiting);
                        if let Some(cmd) = cmd_rx.blocking_recv() {
                            if let Err(e) = connection.send(cmd) {
                                let _ = blocking_sender.try_send(Message::DebugDisconnected(
                                    format!("Send error: {e}"),
                                ));
                                break;
                            }
                        } else {
                            let _ = blocking_sender.try_send(Message::DebugDisconnected(
                                "Command channel closed".into(),
                            ));
                            break;
                        }
                    } else if let Err(e) = connection.send(DebugCommand::Ack) {
                        let _ = blocking_sender
                            .try_send(Message::DebugDisconnected(format!("Send error: {e}")));
                        break;
                    }
                }

                Ok(())
            })
            .await;

            if let Ok(Err(e)) = result {
                let _ = sender.try_send(Message::DebugDisconnected(e));
            }

            std::future::pending::<()>().await;
        },
    )
}

// Return addresses and ports to be used for each of the three queues
// - (general) job source
// - context job source
// - results sink
// - control messages
fn get_connect_addresses(ports: (u16, u16, u16, u16)) -> (String, String, String, String) {
    (
        format!("tcp://127.0.0.1:{}", ports.0),
        format!("tcp://127.0.0.1:{}", ports.1),
        format!("tcp://127.0.0.1:{}", ports.2),
        format!("tcp://127.0.0.1:{}", ports.3),
    )
}

// Return addresses to bind to for
// - (general) job source
// - context job source
// - results sink
// - control messages
fn get_bind_addresses(ports: (u16, u16, u16, u16)) -> (String, String, String, String) {
    (
        format!("tcp://*:{}", ports.0),
        format!("tcp://*:{}", ports.1),
        format!("tcp://*:{}", ports.2),
        format!("tcp://*:{}", ports.3),
    )
}

// Return four free ports to use for client-coordinator message queues
fn get_four_ports() -> Result<(u16, u16, u16, u16)> {
    Ok((
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
    ))
}
