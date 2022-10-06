#![allow(missing_docs)]

pub use error_chain::bail;
use error_chain::error_chain;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Url(url::ParseError);
        FlowCore(flowcore::errors::Error);
        Runtime(flowrlib::errors::Error);
        Io(std::io::Error);
    }
}
