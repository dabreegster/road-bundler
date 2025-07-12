use geo::{Coord, Euclidean, InterpolatableLine, Length, Line, LineString};

// TODO Upstream to geo

/// Walks along a linestring at regular intervals and output the point and angle of the line in
/// degrees. Always includes the start and end point. This can't use
/// https://docs.rs/geo/latest/geo/algorithm/line_interpolate_point/trait.LineInterpolatePoint.html
/// because the line / angle isn't returned.
pub fn step_along_line(linestring: &LineString, interval: f64) -> Vec<(Coord, f64)> {
    // TODO This is very inefficient; it keeps searching from the start of the whole linestring
    let mut result = Vec::new();
    let mut dist_along = 0.0;
    let length = Euclidean.length(linestring);
    while dist_along < length {
        result.push(dist_along_linestring(linestring, dist_along));
        dist_along += interval;
    }
    // TODO Or adjust interval... max interval
    if dist_along > length {
        result.push(dist_along_linestring(linestring, length));
    }
    result
}

fn dist_along_linestring(linestring: &LineString, dist: f64) -> (Coord, f64) {
    let mut dist_left = dist;
    for line in linestring.lines() {
        let length = Euclidean.length(&line);
        if length == 0.0 {
            continue;
        }
        if dist_left <= length {
            return (
                line.point_at_distance_from_start(&Euclidean, dist_left)
                    .into(),
                line_angle_degrees(line),
            );
        }
        dist_left -= length;
    }
    // If there's leftover, it's just a small epsilon
    let line = linestring.lines().last().unwrap();
    (line.end, line_angle_degrees(line))
}

fn line_angle_degrees(line: Line) -> f64 {
    line.dy().atan2(line.dx()).to_degrees()
}
