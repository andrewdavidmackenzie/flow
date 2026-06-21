//! functions for chart/visualization generation
//! ## Charts (//flowstdlib/charts)

/// A module that renders an array of numbers as a histogram bar chart image
#[path = "histogram/histogram.rs"]
pub mod histogram;
/// A module that renders an array of values as a time series chart image
#[path = "time_series/time_series.rs"]
pub mod time_series;
