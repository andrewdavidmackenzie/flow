
pub type Route = String;

pub trait HasRoute {
    fn route(&self) -> &Route;
}

pub trait FindRoute {
    fn find(&self, route: &Route) -> bool;
}