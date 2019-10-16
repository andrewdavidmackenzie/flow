use crate::errors::*;

/// A content provider is responsible with interfacing with the environment and doing IO
/// or what is required to supply content related with flows - isolating other libraries
/// from the File SSystem or IO. It must implement the `Provider` trait
pub trait Provider {
    /// Take a URL and uses it to determine a url where actual content can be read from
    /// using some provider specific logic. This may involve looking for default files in a
    /// directory (a file provider) or a server path (an http provider), or it may involve
    /// translating a virtual URL into a real on where content can be found (lib provider).
    /// It also returns an optional String which is a library reference in case that applies.
    fn resolve_url(&self, url: &str, default_file: &str, extensions: &[&str]) -> Result<(String, Option<String>)>;

    /// Fetches content from a URL. It resolves the URL internally before attempting to
    /// fetch actual content
    fn get_contents(&self, url: &str) -> Result<Vec<u8>>;
}