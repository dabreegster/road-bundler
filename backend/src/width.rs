use anyhow::Result;
use geo::{Coord, Densify, Euclidean, Length, Line, LineIntersection, LineString, Polygon};
use rstar::{RTree, RTreeObject};

use crate::{Debugger, EdgeID, RoadBundler};

pub fn debug_road_width(bundler: &RoadBundler, e: EdgeID) -> Result<String> {
    let step_size_meters = 10.0;
    let project_away_meters = 50.0;

    let test_points = points_along_line(&bundler.graph.edges[&e].linestring, step_size_meters);
    let mut num_perps = 0;
    let mut num_hit_checks = 0;
    let mut debugger = Debugger::new(bundler.graph.mercator.clone());
    for (pt, angle) in test_points {
        num_perps += 1;

        let mut test_lines = Vec::new();
        for angle_offset in [-90.0, 90.0] {
            let projected = project_away(pt, angle + angle_offset, project_away_meters);
            let full_line = Line::new(pt, projected);

            test_lines.extend(shortest_line_hitting_polygon(
                full_line,
                &bundler.buildings,
                &mut num_hit_checks,
            ));
        }
        // If either of the test lines doesn't hit anything within project_away_meters, then
        // something's probably wrong -- skip it as output
        if test_lines.len() != 2 {
            continue;
        }
        let full_line = LineString::new(vec![test_lines[0].end, test_lines[1].end]);
        debugger.line(&full_line, "test line", "red", 2, 1.0);
    }
    info!(
        "Tried {} perpendiculars, with a total of {} line hit checks",
        num_perps, num_hit_checks
    );
    Ok(serde_json::to_string(&debugger.build())?)
}

// TODO Move to geo_helpers...
// Every step_size along a LineString, returns the point and angle
fn points_along_line(linestring: &LineString, step_size_meters: f64) -> Vec<(Coord, f64)> {
    let mut result = Vec::new();
    // Using lines instead of coords so we can get the angle -- but is this hard to reason about?
    for line in Euclidean.densify(linestring, step_size_meters).lines() {
        // TODO For the last line, use the last point too
        let pt = line.start;
        let angle = line_angle_degrees(line);
        result.push((pt, angle));
    }
    result
}

fn line_angle_degrees(line: Line) -> f64 {
    line.dy().atan2(line.dx()).to_degrees()
}

fn project_away(pt: Coord, angle_degrees: f64, distance: f64) -> Coord {
    let (sin, cos) = angle_degrees.to_radians().sin_cos();
    Coord {
        x: pt.x + distance * cos,
        y: pt.y + distance * sin,
    }
}

// Assuming line.start is outside all of the polygons, looks for all possible intersections between
// the line and a polygon, and trims the line back to the edge of the nearest polygon
fn shortest_line_hitting_polygon(
    line: Line,
    rtree: &RTree<Polygon>,
    num_hit_checks: &mut usize,
) -> Option<Line> {
    let mut shortest: Option<(Line, f64)> = None;
    for polygon in rtree.locate_in_envelope_intersecting(&line.envelope()) {
        // Ignore polygon holes
        for polygon_line in polygon.exterior().lines() {
            *num_hit_checks += 1;
            if let Some(LineIntersection::SinglePoint { intersection, .. }) =
                geo::algorithm::line_intersection::line_intersection(line, polygon_line)
            {
                let candidate = Line::new(line.start, intersection);
                let candidate_length = Euclidean.length(&candidate);
                if shortest
                    .as_ref()
                    .map(|(_, len)| candidate_length < *len)
                    .unwrap_or(true)
                {
                    shortest = Some((candidate, candidate_length));
                }
            }
        }
    }
    shortest.map(|pair| pair.0)
}
