#![allow(missing_docs)]

pub use error_chain::bail;
use error_chain::error_chain;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Core(flowcore::errors::Error);
        Compiler(flowrclib::errors::Error);
        Io(std::io::Error);
        Url(url::ParseError);
        Toml(toml::de::Error);
    }
}
