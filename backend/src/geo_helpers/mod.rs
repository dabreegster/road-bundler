// TODO Consider upstreaming all of these

mod average_lines;
mod slice_nearest_boundary;

pub use average_lines::average_linestrings;
pub use slice_nearest_boundary::SliceNearEndpoints;

use geo::{Coord, Line, LineString};

/// The bearing from the linestring's start to end
pub fn linestring_bearing(linestring: &LineString) -> f64 {
    let pt1 = linestring.0[0];
    let pt2 = linestring.0[linestring.0.len() - 1];
    euclidean_bearing(pt1, pt2)
}

fn angle_of_line(line: Line) -> f64 {
    (line.dy()).atan2(line.dx()).to_degrees()
}

/// North is 0°
/// East is 90°
/// South  is 180°
/// West is 270°
fn euclidean_bearing(origin: Coord, destination: Coord) -> f64 {
    (angle_of_line(Line::new(origin, destination)) + 450.0) % 360.0
}
