#![allow(missing_docs)]

pub use error_chain::bail;
use error_chain::error_chain;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Url(url::ParseError);
        Io(std::io::Error);
        Serde(serde_json::error::Error);
        Recv(std::sync::mpsc::RecvError);
    }
}
