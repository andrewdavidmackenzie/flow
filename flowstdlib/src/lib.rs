extern crate flowrlib;

pub mod stdio;
pub mod info;

include!(concat!(env!("OUT_DIR"), "/manifest.rs"));

pub fn get_message()  -> &'static str {
    message()
}