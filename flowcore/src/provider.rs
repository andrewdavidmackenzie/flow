use url::Url;

use crate::errors::Result;

/// A content provider is responsible with interfacing with the environment and doing IO
/// or what is required to supply content related with flows - isolating other libraries
/// from the File System or IO. It must implement the `Provider` trait
pub trait Provider: Sync + Send {
    /// Take a URL and uses it to determine a url where actual content can be read from
    /// using some provider specific logic. This may involve looking for default files in a
    /// directory (a file provider) or a server path (an http provider), or it may involve
    /// translating a library URL into a real on where content can be found.
    ///
    /// # Errors
    ///
    /// Returns an error if the `Provider` cannot determine the "real `Url`" (where the content
    /// reside) corresponding to this url
    ///
    fn resolve_url(
        &self,
        url: &Url,
        default_name: &str,
        extensions: &[&str],
    ) -> Result<(Url, Option<Url>)>;

    /// Fetches content from a URL. It resolves the URL internally before attempting to
    /// fetch actual content
    ///
    /// # Errors
    ///
    /// Returns an error if the selected `Provider` cannot read the contents from the provided `Url`
    fn get_contents(&self, url: &Url) -> Result<Vec<u8>>;
}