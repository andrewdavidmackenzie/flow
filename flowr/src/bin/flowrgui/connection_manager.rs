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

/// Global storage for a user-selected coordinator address (from the picker)
static DISCOVERED_ADDRESS: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

/// Set a discovered coordinator address for the subscription to pick up
pub fn set_discovered_address(address: String) {
    if let Ok(mut guard) = DISCOVERED_ADDRESS.write() {
        *guard = Some(address);
    }
}

/// Check if a discovered address has been set
pub fn has_discovered_address() -> bool {
    DISCOVERED_ADDRESS.read().is_ok_and(|guard| guard.is_some())
}

/// Take the discovered address (consumes it)
fn take_discovered_address() -> Option<String> {
    if let Ok(mut guard) = DISCOVERED_ADDRESS.write() {
        guard.take()
    } else {
        None
    }
}

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

/// Flag: show only flows in next `ProcessTree` response
#[cfg(feature = "debugger")]
static FLOWS_ONLY: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "debugger")]
/// Set whether next `ProcessTree` should show flows only
pub fn set_flows_only(v: bool) {
    FLOWS_ONLY.store(v, Ordering::Relaxed);
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
            let is_client_only = matches!(settings, CoordinatorSettings::ClientOnly);
            let mut state = match settings {
                CoordinatorSettings::Server(sett) => CoordinatorState::Init(sett.clone()),
                CoordinatorSettings::ClientOnly => CoordinatorState::Discovery,
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
                        if let Some(address) = take_discovered_address() {
                            state = CoordinatorState::Discovered(address);
                        } else if is_client_only {
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        } else {
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
    .replace(
        "Cannot run a process mid-execution. Reset first with 'r'.",
        "Cannot run a process mid-execution. Click Reset first.",
    )
    .replace(
        "'break' command must specify a breakpoint",
        "A breakpoint target must be specified",
    )
    .replace(
        "To break on every Function, you can just single step using 's' command",
        "To break on every Function, use the Step button",
    )
    .replace(
        "Supply them as arguments: r <target> <val1> <val2> ...",
        "Fill in values in the input panel and click Execute.",
    )
}

/// Format a `DebugServerMessage` into colored lines for the debug tab
#[cfg(feature = "debugger")]
fn entity_line(
    func: &flowcore::model::runtime_function::RuntimeFunction,
    indent: &str,
    link_type: crate::LinkType,
    suffix: &str,
) -> crate::DebugEventLine {
    let entity = match link_type {
        crate::LinkType::Flow => "Flow",
        _ => "Function",
    };
    crate::DebugLineBuilder::new()
        .text(indent)
        .chip(
            &format!("{entity} #{}", func.id()),
            &func.id().to_string(),
            link_type,
        )
        .text(&format!(" '{}' @ ", func.name()))
        .chip(func.route(), func.route(), crate::LinkType::Route)
        .text(suffix)
        .finish()
}

#[cfg(feature = "debugger")]
#[allow(clippy::too_many_lines)]
fn format_debug_event(message: &DebugServerMessage) -> Vec<crate::DebugEventLine> {
    use crate::theme::debug_colors;
    use crate::DebugEventLine;

    let line = |text: String, color: Option<iced::Color>| vec![DebugEventLine::new(text, color)];

    match message {
        DebugServerMessage::JobCompleted(job) => {
            let mut lines = vec![DebugEventLine::new(
                format!(
                    "Job #{} completed by Function #{} (Flow #{}) '{}'",
                    job.payload.job_id, job.process_id, job.parent_id, job.function_name
                ),
                Some(debug_colors::COMPLETION),
            )];
            if let Ok((Some(output), _)) = &job.result {
                lines.push(DebugEventLine::new(
                    format!("\tOutput value: '{output}'"),
                    Some(debug_colors::COMPLETION),
                ));
            }
            lines
        }
        DebugServerMessage::PriorToSendingJob(job) => vec![
            DebugEventLine::new(
                format!(
                    "About to send Job #{} to Function #{} (Flow #{}) '{}'",
                    job.payload.job_id, job.process_id, job.parent_id, job.function_name
                ),
                Some(debug_colors::DATA_FLOW),
            ),
            DebugEventLine::new(
                format!("Inputs: {:?}", job.payload.input_set),
                Some(debug_colors::DATA_FLOW),
            ),
        ],
        DebugServerMessage::DataBreakpoint(
            source_name,
            source_id,
            output_route,
            value,
            dest_id,
            dest_name,
            io_name,
            input_number,
        ) => line(
            format!(
                "Data breakpoint: Function #{source_id} '{source_name}{output_route}' \
                --{value}-> Function #{dest_id}:{input_number} '{dest_name}'/'{io_name}'"
            ),
            Some(debug_colors::BREAKPOINT),
        ),
        DebugServerMessage::Panic(message, jobs_created) => line(
            format!("Function panicked after {jobs_created} jobs created: {message}"),
            Some(debug_colors::ERROR),
        ),
        DebugServerMessage::JobError(job) => line(
            format!(
                "Error executing Job #{} on Function #{} (Flow #{}) '{}': '{job}'",
                job.payload.job_id, job.process_id, job.parent_id, job.function_name
            ),
            Some(debug_colors::ERROR),
        ),
        DebugServerMessage::Deadlock(message) => line(
            format!("Deadlock detected: {message}"),
            Some(debug_colors::ERROR),
        ),
        DebugServerMessage::EnteringDebugger => line(
            "Entering debugger. Use the controls above to debug.".into(),
            Some(debug_colors::STATUS),
        ),
        DebugServerMessage::ExitingDebugger => {
            line("Debugger is exiting".into(), Some(debug_colors::STATUS))
        }
        DebugServerMessage::ExecutionStarted => {
            line("Running flow".into(), Some(debug_colors::STATUS))
        }
        DebugServerMessage::ExecutionEnded => {
            line("Flow has completed".into(), Some(debug_colors::COMPLETION))
        }
        DebugServerMessage::Functions(ref functions) => functions
            .iter()
            .map(|f| entity_line(f, "  ", crate::LinkType::Function, ""))
            .collect(),
        DebugServerMessage::SendingValue(source_id, value, dest_id, input_number) => line(
            format!("Function #{source_id} sending '{value}' to {dest_id}:{input_number}"),
            Some(debug_colors::DATA_FLOW),
        ),
        DebugServerMessage::Error(ref msg) => line(rewrite_for_gui(msg), Some(debug_colors::ERROR)),
        DebugServerMessage::Message(msg) => line(rewrite_for_gui(msg), None),
        DebugServerMessage::Resetting => line("Resetting state".into(), Some(debug_colors::STATUS)),
        DebugServerMessage::FunctionStates((function, states)) => {
            vec![entity_line(
                function,
                "",
                crate::LinkType::Function,
                &format!(" {states:?}"),
            )]
        }
        DebugServerMessage::OverallState(ref run_state) => format_state_only(run_state),
        DebugServerMessage::InputState(input) => {
            let name = input.name();
            let name_label = if name.is_empty() {
                String::new()
            } else {
                format!("'{name}' ")
            };
            if input.is_empty() {
                vec![DebugEventLine::new(
                    format!("Input {name_label}— no values queued"),
                    None,
                )]
            } else {
                vec![DebugEventLine::new(
                    format!(
                        "Input {name_label}— {} value(s) queued: {}",
                        input.values_available(),
                        format!("{input}").trim()
                    ),
                    None,
                )]
            }
        }
        DebugServerMessage::OutputState(connections) => {
            if connections.is_empty() {
                line("No output connections from that sub-route".into(), None)
            } else {
                connections
                    .iter()
                    .map(|c| DebugEventLine::new(format!("{c}"), None))
                    .collect()
            }
        }
        DebugServerMessage::FlowUnblockBreakpoint(flow_id) => line(
            format!("Flow #{flow_id} was busy and has now gone idle, unblocking senders"),
            Some(debug_colors::STATUS),
        ),
        DebugServerMessage::WaitingForCommand(job_id) => line(
            format!("Waiting for command (Job #{job_id})"),
            Some(debug_colors::STATUS),
        ),
        DebugServerMessage::ProcessTree(ref state) => {
            if FLOWS_ONLY.swap(false, Ordering::Relaxed) {
                format_flows_only(state)
            } else {
                format_tree_only(state)
            }
        }
        DebugServerMessage::InspectByState(ref state_name, ref state) => {
            format_inspect_by_state(state_name, state)
        }
        DebugServerMessage::InspectFlow(flow_id, ref state) => format_inspect_flow(*flow_id, state),
        DebugServerMessage::Invalid => line(
            "Invalid message from debug server".into(),
            Some(debug_colors::ERROR),
        ),
        DebugServerMessage::BreakpointList(specs) => {
            if specs.is_empty() {
                line("No breakpoints set".into(), None)
            } else {
                let mut lines = Vec::new();
                for spec in specs {
                    let text = match spec {
                        flowcore::model::debug_command::BreakpointSpec::Numeric(id) => {
                            format!("  Function #{id}")
                        }
                        flowcore::model::debug_command::BreakpointSpec::Completed(id) => {
                            format!("  Function #{id}+ (completion)")
                        }
                        flowcore::model::debug_command::BreakpointSpec::Input((id, num)) => {
                            format!("  Input #{id}:{num}")
                        }
                        flowcore::model::debug_command::BreakpointSpec::Output((id, route)) => {
                            format!("  Output #{id}{route}")
                        }
                        flowcore::model::debug_command::BreakpointSpec::Route(route) => {
                            format!("  Route {route}")
                        }
                        flowcore::model::debug_command::BreakpointSpec::All => String::new(),
                    };
                    if !text.is_empty() {
                        lines.push(DebugEventLine::new(text, None));
                    }
                }
                lines
            }
        }
    }
}

/// Create a subscription that runs a debug client, connecting to the debug server via ZMQ
#[cfg(feature = "debugger")]
pub fn debug_client_subscribe(address: String) -> Subscription<Message> {
    Subscription::run_with(address, |address| debug_client_stream(address.clone()))
}

#[cfg(feature = "debugger")]
#[allow(clippy::too_many_lines)]
fn debug_client_stream(address: String) -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(
        100,
        move |mut sender: iced::futures::channel::mpsc::Sender<Message>| async move {
            let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::channel::<DebugCommand>(10);
            if let Ok(mut guard) = DEBUG_CMD_SENDER.write() {
                *guard = Some(cmd_tx);
            }

            let mut blocking_sender = sender.clone();
            let result = tokio::task::spawn_blocking(move || -> std::result::Result<(), String> {
                let address = address.clone();
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

                    if let DebugServerMessage::Functions(ref functions) = message {
                        let func_data: Vec<crate::CachedFunction> = functions
                            .iter()
                            .map(|f| {
                                let inputs: Vec<(usize, String, bool)> = f
                                    .inputs()
                                    .iter()
                                    .enumerate()
                                    .map(|(i, inp)| (i, inp.name().to_string(), inp.is_generic()))
                                    .collect();
                                let mut outputs: Vec<String> = Vec::new();
                                for conn in f.get_output_connections() {
                                    if let flowcore::model::output_connection::Source::Output(
                                        ref route,
                                    ) = conn.source
                                    {
                                        let route = if route.starts_with('/') {
                                            route.clone()
                                        } else {
                                            format!("/{route}")
                                        };
                                        if !outputs.contains(&route) {
                                            outputs.push(route);
                                        }
                                    }
                                }
                                crate::CachedFunction {
                                    id: f.id(),
                                    name: f.name().to_string(),
                                    route: f.route().to_string(),
                                    inputs,
                                    outputs,
                                    is_flow: false,
                                }
                            })
                            .collect();
                        let _ =
                            blocking_sender.try_send(Message::DebugFunctionListReceived(func_data));
                    }

                    if let DebugServerMessage::BreakpointList(ref specs) = message {
                        let spec_strings: Vec<String> = specs
                            .iter()
                            .map(|s| match s {
                                flowcore::model::debug_command::BreakpointSpec::Numeric(id) => {
                                    format!("{id}")
                                }
                                flowcore::model::debug_command::BreakpointSpec::Completed(id) => {
                                    format!("{id}+")
                                }
                                flowcore::model::debug_command::BreakpointSpec::Input((
                                    id,
                                    num,
                                )) => {
                                    format!("{id}:{num}")
                                }
                                flowcore::model::debug_command::BreakpointSpec::Output((
                                    id,
                                    route,
                                )) => format!("{id}{route}"),
                                flowcore::model::debug_command::BreakpointSpec::Route(route) => {
                                    route.clone()
                                }
                                flowcore::model::debug_command::BreakpointSpec::All => {
                                    String::new()
                                }
                            })
                            .filter(|s| !s.is_empty())
                            .collect();
                        let _ = blocking_sender
                            .try_send(Message::DebugBreakpointListReceived(spec_strings));
                    }

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

#[cfg(feature = "debugger")]
fn format_tree_only(run_state: &flowrlib::run_state::RunState) -> Vec<crate::DebugEventLine> {
    let mut lines = format_run_state(run_state);
    if let Some(pos) = lines.iter().position(|l| l.text == "RunState:") {
        lines.truncate(pos);
    }
    lines
}

#[cfg(feature = "debugger")]
fn format_flows_only(run_state: &flowrlib::run_state::RunState) -> Vec<crate::DebugEventLine> {
    use crate::{DebugLineBuilder, LinkType};
    let manifest = &run_state.get_submission().manifest;
    let functions = run_state.get_functions();
    if manifest.flows().is_empty() {
        return vec![crate::DebugEventLine::new("No flows".into(), None)];
    }
    let mut lines = Vec::new();
    let mut sorted_flows: Vec<_> = manifest.flows().iter().collect();
    sorted_flows.sort_by_key(|(id, _)| *id);
    for (id, flow) in sorted_flows {
        let mut b = DebugLineBuilder::new().text("  ");
        b = if let Some(func) = functions.get(id) {
            b.chip(&format!("flow #{id}"), &id.to_string(), LinkType::Flow)
                .text(&format!(" '{}' @ ", func.name()))
                .chip(func.route(), func.route(), LinkType::Route)
        } else {
            b.chip(&format!("flow #{id}"), &id.to_string(), LinkType::Flow)
        };
        if flow.parent_id.is_none() {
            b = b.text(" (root)");
        }
        if !flow.sub_flow_ids.is_empty() {
            b = b.text(" sub-flows: [");
            for (i, sf) in flow.sub_flow_ids.iter().enumerate() {
                if i > 0 {
                    b = b.text(", ");
                }
                b = if let Some(func) = functions.get(sf) {
                    b.chip(
                        &format!("flow #{sf} '{}'", func.name()),
                        &sf.to_string(),
                        LinkType::Flow,
                    )
                } else {
                    b.chip(&format!("flow #{sf}"), &sf.to_string(), LinkType::Flow)
                };
            }
            b = b.text("]");
        }
        lines.push(b.finish());
    }
    lines
}

#[cfg(feature = "debugger")]
fn format_state_only(run_state: &flowrlib::run_state::RunState) -> Vec<crate::DebugEventLine> {
    let lines = format_run_state(run_state);
    if let Some(pos) = lines.iter().position(|l| l.text == "RunState:") {
        lines.into_iter().skip(pos + 1).collect()
    } else {
        vec![]
    }
}

#[cfg(feature = "debugger")]
#[allow(clippy::too_many_lines)]
fn format_run_state(run_state: &flowrlib::run_state::RunState) -> Vec<crate::DebugEventLine> {
    use crate::{DebugEventLine, DebugLineBuilder, LinkType};
    use std::collections::BTreeMap;

    fn add_flow_tree(
        lines: &mut Vec<crate::DebugEventLine>,
        by_parent: &BTreeMap<usize, Vec<&flowcore::model::runtime_function::RuntimeFunction>>,
        run_state: &flowrlib::run_state::RunState,
        flow_id: usize,
        ancestors: &[crate::TreeSegment],
        connector: Option<crate::TreeSegment>,
    ) {
        use crate::TreeSegment;
        let manifest = &run_state.get_submission().manifest;
        let functions = run_state.get_functions();

        // Build this node's prefix: ancestors + own connector
        let mut prefix: Vec<TreeSegment> = ancestors.to_vec();
        if let Some(conn) = connector {
            prefix.push(conn);
        }

        // Emit the flow node itself as a collapsible separator
        let mut flow_line = if let Some(func) = functions.get(&flow_id) {
            entity_line(func, "", crate::LinkType::Flow, "")
        } else {
            crate::DebugLineBuilder::new()
                .chip(
                    &format!("flow #{flow_id}"),
                    &flow_id.to_string(),
                    crate::LinkType::Flow,
                )
                .finish()
        };
        flow_line.separator = true;
        flow_line.tree_prefix = prefix;
        lines.push(flow_line);

        // Build child ancestor prefix: Branch → Pipe, End → Space
        let mut child_ancestors: Vec<TreeSegment> = ancestors.to_vec();
        if let Some(conn) = connector {
            child_ancestors.push(match conn {
                TreeSegment::Branch => TreeSegment::Pipe,
                TreeSegment::End => TreeSegment::Space,
                other => other,
            });
        }

        // Collect all children: sub-flows first, then functions
        let sub_ids = manifest
            .flows()
            .get(&flow_id)
            .map(|fi| {
                let mut ids = fi.sub_flow_ids.clone();
                ids.sort();
                ids
            })
            .unwrap_or_default();
        let func_children: Vec<_> = by_parent
            .get(&flow_id)
            .map(|children| children.iter().filter(|c| c.id() != flow_id).collect())
            .unwrap_or_default();
        let total_children = sub_ids.len() + func_children.len();

        // Emit sub-flows
        let mut child_idx = 0;
        for sub_id in &sub_ids {
            child_idx += 1;
            let is_last = child_idx == total_children;
            let conn = if is_last {
                TreeSegment::End
            } else {
                TreeSegment::Branch
            };
            add_flow_tree(
                lines,
                by_parent,
                run_state,
                *sub_id,
                &child_ancestors,
                Some(conn),
            );
        }

        // Emit direct function children
        for child in &func_children {
            child_idx += 1;
            let is_last = child_idx == total_children;
            let conn = if is_last {
                TreeSegment::End
            } else {
                TreeSegment::Branch
            };
            let states = run_state.get_function_states(child.id());
            let mut line = entity_line(
                child,
                "",
                crate::LinkType::Function,
                &format!(" {states:?}"),
            );
            let mut child_prefix = child_ancestors.clone();
            child_prefix.push(conn);
            line.tree_prefix = child_prefix;
            lines.push(line);
        }
    }

    let mut lines = Vec::new();
    let manifest = &run_state.get_submission().manifest;

    // Process tree — flows as collapsible nodes containing their functions
    lines.push(DebugEventLine::new("Process Tree:".into(), None));
    let functions = run_state.get_functions();
    let mut by_parent: BTreeMap<usize, Vec<&flowcore::model::runtime_function::RuntimeFunction>> =
        BTreeMap::new();
    for func in functions.values() {
        // Skip functions that are also flows — they appear as flow nodes
        if !manifest.flows().contains_key(&func.id()) {
            by_parent
                .entry(func.get_parent_id())
                .or_default()
                .push(func);
        }
    }
    for children in by_parent.values_mut() {
        children.sort_by_key(|f| f.id());
    }

    // Find root flow(s) and walk the hierarchy
    let mut root_flows: Vec<usize> = manifest
        .flows()
        .iter()
        .filter(|(_, info)| info.parent_id.is_none())
        .map(|(id, _)| *id)
        .collect();
    root_flows.sort();
    for root_id in &root_flows {
        add_flow_tree(&mut lines, &by_parent, run_state, *root_id, &[], None);
    }

    // RunState stats
    lines.push(DebugEventLine::new("RunState:".into(), None));
    lines.push(DebugEventLine::new(
        format!("  Jobs Created: {}", run_state.get_number_of_jobs_created()),
        None,
    ));

    // Running jobs
    let running = run_state.get_running();
    let mut b = DebugLineBuilder::new().text("  ").chip(
        &format!("Jobs Running ({}):", running.len()),
        "running",
        LinkType::State,
    );
    if !running.is_empty() {
        let mut sorted_running: Vec<_> = running.iter().collect();
        sorted_running.sort_by_key(|(id, _)| *id);
        for (job_id, job) in &sorted_running {
            b = b.text(" ").chip(
                &format!("Job #{job_id}"),
                &job.process_id.to_string(),
                LinkType::Job,
            );
        }
    }
    lines.push(b.finish());

    // Ready jobs
    let ready = run_state.get_ready_jobs();
    let mut b = DebugLineBuilder::new().text("  ").chip(
        &format!("Jobs Ready ({}):", ready.len()),
        "ready",
        LinkType::State,
    );
    for job in ready {
        b = b.text(" ").chip(
            &format!("Job #{}", job.payload.job_id),
            &job.process_id.to_string(),
            LinkType::Job,
        );
    }
    lines.push(b.finish());

    // Completed functions
    let completed = run_state.get_completed();
    let mut b = DebugLineBuilder::new().text("  ").chip(
        &format!("Functions Completed ({}):", completed.len()),
        "completed",
        LinkType::State,
    );
    if !completed.is_empty() {
        let mut sorted: Vec<_> = completed.iter().collect();
        sorted.sort();
        for id in &sorted {
            b = b
                .text(" ")
                .chip(&format!("#{id}"), &id.to_string(), LinkType::Function);
        }
        b = b.text("]");
    }
    lines.push(b.finish());

    // Busy functions and flows (separated)
    let busy = run_state.get_busy_count();
    let flows_map = manifest.flows();
    let mut busy_flows = Vec::new();
    let mut busy_funcs = Vec::new();
    for (id, count) in busy {
        if flows_map.contains_key(id) {
            busy_flows.push((*id, *count));
        } else {
            busy_funcs.push((*id, *count));
        }
    }

    busy_flows.sort_by_key(|(id, _)| *id);
    busy_funcs.sort_by_key(|(id, _)| *id);

    if !busy_flows.is_empty() {
        let mut b = DebugLineBuilder::new().text("  Busy Flows: ");
        for (i, (id, _count)) in busy_flows.iter().enumerate() {
            if i > 0 {
                b = b.text(", ");
            }
            b = b.chip(&format!("Flow #{id}"), &id.to_string(), LinkType::Flow);
        }
        lines.push(b.finish());
    }

    if !busy_funcs.is_empty() {
        let mut b = DebugLineBuilder::new().text("  Busy Functions: ");
        for (i, (id, _count)) in busy_funcs.iter().enumerate() {
            if i > 0 {
                b = b.text(", ");
            }
            b = b.chip(
                &format!("Function #{id}"),
                &id.to_string(),
                LinkType::Function,
            );
        }
        lines.push(b.finish());
    }

    lines
}

#[cfg(feature = "debugger")]
fn format_inspect_by_state(
    state_name: &str,
    run_state: &flowrlib::run_state::RunState,
) -> Vec<crate::DebugEventLine> {
    use crate::DebugEventLine;

    let target_state = match state_name {
        "ready" => Some(flowrlib::run_state::State::Ready),
        "waiting" => Some(flowrlib::run_state::State::Waiting),
        "running" => Some(flowrlib::run_state::State::Running),
        "completed" => Some(flowrlib::run_state::State::Completed),
        "blocked" => None,
        _ => {
            return vec![DebugEventLine::new(
                format!("Unknown state '{state_name}'"),
                Some(crate::theme::debug_colors::ERROR),
            )];
        }
    };

    let mut lines = Vec::new();
    let functions = run_state.get_functions();
    let mut sorted: Vec<_> = functions.values().collect();
    sorted.sort_by_key(|f| f.id());

    if let Some(ref target) = target_state {
        lines.push(DebugEventLine::new(
            format!("Functions in '{state_name}' state:"),
            None,
        ));
        let mut count = 0;
        for func in &sorted {
            let states = run_state.get_function_states(func.id());
            if states.contains(target) {
                count += 1;
                lines.push(entity_line(func, "  ", crate::LinkType::Function, ""));
            }
        }
        if count == 0 {
            lines.push(DebugEventLine::new("  (none)".into(), None));
        }
    } else {
        lines.push(DebugEventLine::new("Blocked functions:".into(), None));
        let mut count = 0;
        for func in &sorted {
            let states = run_state.get_function_states(func.id());
            if states.contains(&flowrlib::run_state::State::Waiting) {
                if let Ok(blockers) = run_state.get_input_blockers(func.id()) {
                    if !blockers.is_empty() {
                        count += 1;
                        lines.push(entity_line(
                            func,
                            "  ",
                            crate::LinkType::Function,
                            &format!(" — blocked by: {blockers:?}"),
                        ));
                    }
                }
            }
        }
        if count == 0 {
            lines.push(DebugEventLine::new("  (none)".into(), None));
        }
    }
    lines
}

#[cfg(feature = "debugger")]
fn format_inspect_flow(
    flow_id: usize,
    run_state: &flowrlib::run_state::RunState,
) -> Vec<crate::DebugEventLine> {
    use crate::{DebugEventLine, DebugLineBuilder, LinkType};

    let mut lines = Vec::new();
    let functions = run_state.get_functions();
    let manifest = &run_state.get_submission().manifest;

    if let Some(func) = functions.get(&flow_id) {
        lines.push(entity_line(func, "", LinkType::Flow, ""));
    }

    if let Some(flow_info) = manifest.flows().get(&flow_id) {
        if let Some(parent) = flow_info.parent_id {
            let mut b = DebugLineBuilder::new().text("  Parent: ");
            if let Some(pf) = functions.get(&parent) {
                b = b.chip(
                    &format!("Flow #{parent} '{}'", pf.name()),
                    &parent.to_string(),
                    LinkType::Flow,
                );
            } else {
                b = b.chip(
                    &format!("Flow #{parent}"),
                    &parent.to_string(),
                    LinkType::Flow,
                );
            }
            lines.push(b.finish());
        }

        if !flow_info.sub_flow_ids.is_empty() {
            let mut b = DebugLineBuilder::new().text("  Sub-flows: ");
            for (i, sf) in flow_info.sub_flow_ids.iter().enumerate() {
                if i > 0 {
                    b = b.text(", ");
                }
                if let Some(sf_func) = functions.get(sf) {
                    b = b.chip(
                        &format!("Flow #{sf} '{}'", sf_func.name()),
                        &sf.to_string(),
                        LinkType::Flow,
                    );
                } else {
                    b = b.chip(&format!("Flow #{sf}"), &sf.to_string(), LinkType::Flow);
                }
            }
            lines.push(b.finish());
        }
    }

    let mut funcs: Vec<_> = functions
        .values()
        .filter(|f| {
            f.get_parent_id() == flow_id
                && f.id() != flow_id
                && !manifest.flows().contains_key(&f.id())
        })
        .collect();
    funcs.sort_by_key(|f| f.id());
    if !funcs.is_empty() {
        lines.push(DebugEventLine::new("  Functions:".into(), None));
        for func in funcs {
            let states = run_state.get_function_states(func.id());
            lines.push(entity_line(
                func,
                "    ",
                LinkType::Function,
                &format!(" {states:?}"),
            ));
        }
    }

    lines
}

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
