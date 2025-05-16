#![allow(missing_docs)]
#![allow(unexpected_cfgs)]

pub use error_chain::bail;
use error_chain::error_chain;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Url(url::ParseError);
        Conversion(std::num::TryFromIntError);
    }
}
