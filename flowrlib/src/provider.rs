pub trait Provider {
    /// 'resolve' takes a Url and uses it to determine a url where actual content can be read from
    /// using some provider specific logic. This may involve looking for default files in a
    /// directory (a file provider) or a server path (an http provider), or it may involve
    /// translating a virtual Url into a real on where content can be found (lib provider).
    /// It also returns an optional String which is a library reference in case that applies.
    fn resolve(&self, url: &str) -> Result<(String, Option<String>), String>;

    /// 'get' fetches content from a url. It resolves the url internally before attempting to
    /// fetch actual content
    fn get(&self, url: &str) -> Result<String, String>;
}