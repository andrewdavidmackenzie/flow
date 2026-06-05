#[allow(clippy::module_name_repetitions)]
pub mod cli_client;
#[cfg(feature = "submission")]
#[allow(clippy::module_name_repetitions)]
pub mod cli_submission_handler;
pub mod coordinator_message;
pub(crate) mod test_helper;
