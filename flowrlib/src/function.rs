
pub trait Function {
    fn define() -> &'static str where Self: Sized;
}