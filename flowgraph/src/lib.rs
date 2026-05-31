//! `flowgraph` provides graph layout and SVG rendering for flow programs.
//!
//! It computes topological layouts for flow diagrams and renders them as
//! interactive SVG files with clickable navigation and tooltips.

pub mod edge;
pub mod layout;
pub mod renderer;
pub mod shapes;
pub mod style;
