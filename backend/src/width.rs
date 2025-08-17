use anyhow::Result;
use geo::buffer::{BufferStyle, LineCap};
use geo::{Buffer, Coord, Euclidean, Length, Line, LineIntersection, LineString, Polygon};
use geojson::GeoJson;
use rstar::{RTree, RTreeObject};

use crate::{Debugger, EdgeID, RoadBundler};

pub fn debug_road_width(bundler: &RoadBundler, e: EdgeID) -> Result<String> {
    let mut debugger = Debugger::new(bundler.graph.mercator.clone());
    for ls in get_road_widths(bundler, e) {
        debugger.line(&ls, "test line", "red", 2, 1.0);
    }
    Ok(serde_json::to_string(&debugger.build())?)
}

pub fn get_all_road_widths(bundler: &RoadBundler) -> Result<String> {
    let mut features = Vec::new();
    for (id, edge) in &bundler.graph.edges {
        let lines = get_road_widths(bundler, *id);
        let widths: Vec<f64> = lines.into_iter().map(|ls| Euclidean.length(&ls)).collect();
        if let (Some(min), Some(max)) = (
            widths.iter().min_by_key(round),
            widths.iter().max_by_key(round),
        ) {
            let buffered = edge
                .linestring
                .buffer_with_style(BufferStyle::new(*min / 2.0).line_cap(LineCap::Square));
            let mut f = bundler.graph.mercator.to_wgs84_gj(&buffered);
            f.set_property("min_width", *min);
            f.set_property("max_width", *max);
            features.push(f);
        }
    }
    Ok(serde_json::to_string(&GeoJson::from(features))?)
}

fn round(x: &&f64) -> usize {
    (*x * 1000.) as usize
}

fn get_road_widths(bundler: &RoadBundler, e: EdgeID) -> Vec<LineString> {
    let step_size_meters = 10.0;
    let project_away_meters = 50.0;

    let test_points = utils::step_along_line(&bundler.graph.edges[&e].linestring, step_size_meters);
    let mut output = Vec::new();
    for (pt, angle) in test_points {
        let mut test_lines = Vec::new();
        for angle_offset in [-90.0, 90.0] {
            let projected = project_away(pt, angle + angle_offset, project_away_meters);
            let full_line = Line::new(pt, projected);

            test_lines.extend(shortest_line_hitting_polygon(
                full_line,
                vec![
                    &bundler.areas.building_polygons,
                    &bundler.areas.other_polygons,
                ],
            ));
        }
        // If either of the test lines doesn't hit anything within project_away_meters, then
        // something's probably wrong -- skip it as output
        if test_lines.len() != 2 {
            continue;
        }
        output.push(LineString::new(vec![test_lines[0].end, test_lines[1].end]));
    }
    output
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
fn shortest_line_hitting_polygon(line: Line, rtrees: Vec<&RTree<Polygon>>) -> Option<Line> {
    let mut shortest: Option<(Line, f64)> = None;
    for rtree in rtrees {
        for polygon in rtree.locate_in_envelope_intersecting(&line.envelope()) {
            // Ignore polygon holes
            for polygon_line in polygon.exterior().lines() {
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
    }
    shortest.map(|pair| pair.0)
}
