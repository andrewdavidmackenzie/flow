#![allow(missing_docs)]
#![allow(unexpected_cfgs)]

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
        Iced(iced::Error);
    }
}
