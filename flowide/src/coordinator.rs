use std::path::PathBuf;
use std::sync::{Arc, mpsc, Mutex};
use std::thread;

use iced::{Subscription, subscription};
use log::{info, trace};
use portpicker::pick_unused_port;
use url::Url;

use flowcore::meta_provider::MetaProvider;
use flowcore::provider::Provider;
use flowrlib::coordinator::Coordinator;
use flowrlib::dispatcher::Dispatcher;
use flowrlib::executor::Executor;
use flowrlib::services::{CONTROL_SERVICE_NAME, JOB_QUEUES_DISCOVERY_PORT, JOB_SERVICE_NAME, RESULTS_JOB_SERVICE_NAME};

use crate::{CoordinatorSettings, gui, Message};
use crate::errors::*;
//use crate::gui::client_connection::ClientConnection;
use crate::gui::client_connection::discover_service;
use crate::gui::coordinator_connection::{COORDINATOR_SERVICE_NAME, DEBUG_SERVICE_NAME,
                                         enable_service_discovery};
use crate::gui::coordinator_connection::CoordinatorConnection;
use crate::gui::coordinator_message::{ClientMessage, CoordinatorMessage};
//use crate::gui::debug_client::DebugClient;
use crate::gui::debug_handler::CliDebugHandler;
use crate::gui::submission_handler::CLISubmissionHandler;

#[derive(Debug, Clone)]
pub(crate) enum GuiCoordinator {
    Unknown,
    Found(mpsc::Sender<ClientMessage>),
}

impl GuiCoordinator {
    /*
    pub(crate) fn debug_client(
        override_args: Arc<Mutex<Vec<String>>>,
        discovery_port: u16,
    ) -> Result<()> {
        trace!("Creating Debug Client");
        let debug_server_address = discover_service(discovery_port,
                                                    DEBUG_SERVICE_NAME)?;
        let debug_client_connection = ClientConnection::new(&debug_server_address)?;
        let debug_client = DebugClient::new(debug_client_connection,
                                            override_args);
        let _ = thread::spawn(move || {
            debug_client.debug_client_loop();
        });

        Ok(())
    }
     */
}

enum State {
    Starting,
    Ready(mpsc::Receiver<ClientMessage>),
}

// Creates an asynchronous worker that sends messages back and forth between the App and
// the Coordinator
pub fn connect(coordinator_settings: CoordinatorSettings) -> Subscription<Message> {
    struct Connect;


/*    /// Create a new runtime client
    pub fn new(connection: ClientConnection) -> Self {
        Client { connection }
    }

    /// Send a message
    pub async fn send(&mut self, message: ClientMessage) -> Result<()> {
        self.connection.send(message)
    }

*/
//    let client_connection = ClientConnection::new(&coordinator_info.0)
//        .unwrap(); // TODO
//    let mut client = Client::new(client_connection);

//    override_args: Arc<Mutex<Vec<String>>>,
//    if debug_this_flow {
//        // TODO the debug client gets a clone of the ref to the args so it can override them
//        let _ = GuiCoordinator::debug_client(override_args,
//                                             coordinator_info.1);
//    }

    let _coordinator_info = start(coordinator_settings);

    subscription::channel(
        std::any::TypeId::of::<Connect>(),
        100,
        |mut output| async move {
            let mut state = State::Starting;

            loop {
                match &mut state {
                    State::Starting => {
                        // Create channel to get messages from the app
                        let (sender, receiver) = mpsc::channel();

                        // Send the sender back to the application so it can send us message
                        let _ = output.try_send(Message::CoordinatorReady(sender));

                        // We are ready to receive messages
                        state = State::Ready(receiver);
                    }
                    State::Ready(_receiver) => {
                        // Need to select from channel and zmq to be able to handle both, or
                        // toggle between them?
                        // TODO wait for a coordinator message OR an App message
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                        // Send the message to the App
                        let _ = output.try_send(Message::CoordinatorSent(
                            CoordinatorMessage::Stdout("Hi".to_string())));

                        // read the response
                        let response = _receiver.recv().unwrap(); // TODO
                        println!("App responded with {response}");
                    }
                }
            }
        }
    )
}

fn start(coordinator_settings: CoordinatorSettings) -> (String, u16) {
    let runtime_port = pick_unused_port().chain_err(|| "No ports free").unwrap(); // TODO
    let coordinator_connection = CoordinatorConnection::new(COORDINATOR_SERVICE_NAME,
                                                            runtime_port).unwrap(); // TODO

    let discovery_port = pick_unused_port().chain_err(|| "No ports free").unwrap(); //TODO
    enable_service_discovery(discovery_port, COORDINATOR_SERVICE_NAME, runtime_port)
        .unwrap(); // TODO

    let debug_port = pick_unused_port().chain_err(|| "No ports free").unwrap(); // TODO
    let debug_connection = CoordinatorConnection::new(DEBUG_SERVICE_NAME,
                                                      debug_port).unwrap(); // TODO
    enable_service_discovery(discovery_port, DEBUG_SERVICE_NAME, debug_port)
        .unwrap(); // TODO

    info!("Starting coordinator in background thread");
    thread::spawn(move || {
        let _ = coordinator_main(
            coordinator_settings,
            coordinator_connection,
            debug_connection,
            false,
        );
    });

    let coordinator_address = discover_service(discovery_port, COORDINATOR_SERVICE_NAME)
        .unwrap(); // TODO;

    (coordinator_address, discovery_port)
}

fn coordinator_main(
    coordinator_settings: CoordinatorSettings,
    coordinator_connection: CoordinatorConnection,
    debug_connection: CoordinatorConnection,
    loop_forever: bool,
) -> Result<()> {
    let connection = Arc::new(Mutex::new(coordinator_connection));

    let mut debug_server = CliDebugHandler { debug_server_connection: debug_connection };

    let provider = Arc::new(MetaProvider::new(coordinator_settings.lib_search_path,
                                              PathBuf::from("/"))) as Arc<dyn Provider>;

    let ports = get_four_ports()?;
    trace!("Announcing three job queues and a control socket on ports: {ports:?}");
    let job_queues = get_bind_addresses(ports);
    let dispatcher = Dispatcher::new(job_queues)?;
    enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, JOB_SERVICE_NAME, ports.0)?;
    enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, RESULTS_JOB_SERVICE_NAME, ports.2)?;
    enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, CONTROL_SERVICE_NAME, ports.3)?;

    let (job_source_name, context_job_source_name, results_sink, control_socket) =
        get_connect_addresses(ports);

    let mut executor = Executor::new()?;
    // if the command line options request loading native implementation of available native libs
    // if not, the native implementation is not loaded and later when a flow is loaded it's library
    // references will be resolved and those libraries (WASM implementations) will be loaded at runtime
    if coordinator_settings.native_flowstdlib {
        executor.add_lib(
            flowstdlib::manifest::get_manifest()
                .chain_err(|| "Could not get 'native' flowstdlib manifest")?,
            Url::parse("memory://")? // Statically linked library has no resolved Url
        )?;
    }
    executor.start(provider.clone(), coordinator_settings.num_threads,
                   &job_source_name,
                   &results_sink,
                   &control_socket,
    );

    let mut context_executor = Executor::new()?;
    context_executor.add_lib(
        gui::get_manifest(connection.clone())?,
        Url::parse("memory://")? // Statically linked library has no resolved Url
    )?;
    context_executor.start(provider, 1,
                           &context_job_source_name,
                           &results_sink,
                           &control_socket,
    );

    let mut submitter = CLISubmissionHandler::new(connection);

    let mut coordinator = Coordinator::new(
        dispatcher,
        &mut submitter,
        &mut debug_server
    )?;

    Ok(coordinator.submission_loop(loop_forever)?)
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
    Ok((pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
    ))
}