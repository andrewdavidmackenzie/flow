
pub type Route = String;

pub trait HasRoute {
    fn route(&self) -> &str;
}
