use std::collections::HashSet;

use geo::{Coord, Line, LineString};
use serde::Serialize;
use utils::osm2graph::{EdgeID, Graph};

use crate::Face;

#[derive(Serialize)]
pub struct DualCarriageway {
    pub name: String,
    pub bearings: Vec<f64>,
}

impl DualCarriageway {
    pub fn maybe_new(graph: &Graph, face: &Face) -> Option<Self> {
        // Find all of the oneway edges
        let mut oneways: Vec<EdgeID> = face
            .boundary_edges
            .iter()
            .filter(|e| graph.edges[e].osm_tags.is("oneway", "yes"))
            .cloned()
            .collect();

        // Make sure they have a name
        oneways.retain(|e| graph.edges[e].osm_tags.has("name"));
        if oneways.len() < 2 {
            return None;
        }

        // Do they all have the same name?
        let names: HashSet<String> = oneways
            .iter()
            .map(|e| graph.edges[e].osm_tags.get("name").cloned().unwrap())
            .collect();
        if names.len() != 1 {
            return None;
        }
        let name = names.into_iter().next().unwrap();

        let mut bearings: Vec<f64> = oneways
            .iter()
            .map(|e| linestring_bearing(&graph.edges[e].linestring))
            .collect();
        bearings.sort_by_key(|x| (*x * 1000.0) as usize);

        Some(Self { name, bearings })
    }
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

/// The bearing from the linestring's start to end
fn linestring_bearing(linestring: &LineString) -> f64 {
    let pt1 = linestring.0[0];
    let pt2 = linestring.0[linestring.0.len() - 1];
    euclidean_bearing(pt1, pt2)
}
