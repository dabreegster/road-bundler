use std::collections::HashSet;

use geo::{Coord, Line, LineString};
use geojson::Feature;
use serde::Serialize;
use utils::osm2graph::{EdgeID, Graph};

use crate::Face;

#[derive(Serialize)]
pub struct DualCarriageway {
    pub name: String,
    pub bearings: Vec<f64>,
    pub side1: Feature,
    pub side2: Feature,
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

        let bearings: Vec<f64> = oneways
            .iter()
            .map(|e| linestring_bearing(&graph.edges[e].linestring))
            .collect();

        let clusters = crate::kmeans::kmeans_2(
            &bearings
                .iter()
                .map(|b| {
                    let (y, x) = b.sin_cos();
                    Coord { x, y }
                })
                .collect(),
            100,
        );

        let mut side1 = Vec::new();
        let mut side2 = Vec::new();
        for (e, cluster) in oneways.into_iter().zip(clusters.into_iter()) {
            if cluster == 0 {
                side1.push(crate::join_lines::KeyedLineString {
                    linestring: graph.edges[&e].linestring.clone(),
                    ids: vec![(e, true)],
                });
            } else {
                side2.push(crate::join_lines::KeyedLineString {
                    linestring: graph.edges[&e].linestring.clone(),
                    ids: vec![(e, true)],
                });
            }
        }

        let side1_joined = crate::join_lines::collapse_degree_2(side1);
        let side2_joined = crate::join_lines::collapse_degree_2(side2);
        if side1_joined.len() != 1 || side2_joined.len() != 2 {
            return None;
        }

        Some(Self {
            name,
            bearings,
            side1: graph.mercator.to_wgs84_gj(&side1_joined[0].linestring),
            side2: graph.mercator.to_wgs84_gj(&side2_joined[0].linestring),
        })
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
