// TODO Consider upstreaming all of these

mod average_lines;
mod join_lines;
mod slice_nearest_boundary;

pub use average_lines::average_linestrings;
pub use join_lines::{KeyedLineString, collapse_degree_2};
pub use slice_nearest_boundary::SliceNearEndpoints;
