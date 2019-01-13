use std::borrow::Cow;

pub type Route = String;

pub trait HasRoute {
    fn route(&self) -> &Route;
}

pub trait FindRoute {
    fn find(&self, route: &Route) -> bool;
}

pub trait SetRoute {
    fn set_routes_from_parent(&mut self, parent: &Route, flow_io: bool);
}

pub struct Router;

/*
    return the io route without a trailing number (array index) and if it has one or not

    If the trailing number was present then return the route with a trailing '/'
*/
impl Router {
    // TODO store the route with an indicator it has a trailing array index when created and
    // avoid all this guff
    pub fn without_trailing_array_index(route: &Route) -> (Cow<Route>, usize, bool) {
        let mut parts: Vec<&str> = route.split('/').collect();
        if let Some(last_part) = parts.pop() {
            if let Ok(number) = last_part.parse::<usize>() {
                let route_without_number = parts.join("/");
                return (Cow::Owned(route_without_number), number, true);
            }
        }

        (Cow::Borrowed(route), 0, false)
    }
}