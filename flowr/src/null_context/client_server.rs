/// Structure that holds information about the Server to help clients connect to it
#[derive(Clone)]
pub struct ServerInfo {
}

impl Default for ServerInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerInfo {
    /// Create a new ServerInfo struct
    pub fn new() -> Self {
        ServerInfo {}
    }

    /// Create a ServerInfo struct for the debug service
    #[cfg(feature = "debugger")]
    pub fn debug_info() -> Self {
        ServerInfo::new()
    }
}