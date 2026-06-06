#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
pub mod test {
    use std::sync::{Arc, Mutex};

    use flowrlib::connections::CoordinatorConnection;

    use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};

    pub fn wait_for_then_send(
        wait_for_message: CoordinatorMessage,
        then_send: ClientMessage,
    ) -> Arc<Mutex<CoordinatorConnection>> {
        flowrlib::test_helper::test::wait_for_then_send(
            wait_for_message,
            then_send,
            ClientMessage::Ack,
        )
    }
}
