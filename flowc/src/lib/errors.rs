#![allow(missing_docs)]

pub use error_chain::bail;
use error_chain::error_chain;

// Specify the errors we will produce and foreign links
error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Io(std::io::Error);
        Url(url::ParseError);
        Provider(flowcore::errors::Error);
        GlobWalk(wax::WalkError);
    }
}
