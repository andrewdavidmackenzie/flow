pub trait Provider {
    /// Take a URL and uses it to determine a url where actual content can be read from
    /// using some provider specific logic. This may involve looking for default files in a
    /// directory (a file provider) or a server path (an http provider), or it may involve
    /// translating a virtual URL into a real on where content can be found (lib provider).
    /// It also returns an optional String which is a library reference in case that applies.
    fn resolve(&self, url: &str, default_file: &str) -> Result<(String, Option<String>), String>;

    /// Fetches content from a URL. It resolves the URL internally before attempting to
    /// fetch actual content
    fn get(&self, url: &str) -> Result<Vec<u8>, String>;
}