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
        let clusters = classify_bearings(&bearings);

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

fn classify_bearings(bearings: &Vec<f64>) -> Vec<usize> {
    crate::kmeans::kmeans_2(
        bearings
            .iter()
            .map(|b| {
                let (y, x) = b.sin_cos();
                Coord { x, y }
            })
            .collect(),
        100,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_bearings() {
        for (input, expected) in vec![
            (vec![90, 90, 91, 98, 265, 271], vec![0, 0, 0, 0, 1, 1]),
            (vec![90, 270], vec![0, 1]),
            (vec![85, 90, 90, 90, 270, 274], vec![0, 0, 0, 0, 1, 1]),
            (vec![179, 358, 359, 359, 359, 360], vec![0, 1, 1, 1, 1, 1]),
            // Wrap around angle case
            (vec![1, 179, 184, 352, 353, 359], vec![0, 1, 1, 0, 0, 0]),
        ] {
            let got1 = classify_bearings(&input.iter().map(|b| *b as f64).collect());
            // Clusters 0 and 1 are arbitrary; the opposite is also fine
            let got2: Vec<usize> = got1.iter().map(|c| if *c == 0 { 1 } else { 0 }).collect();
            if got1 != expected && got2 != expected {
                panic!("For bearings {input:?},\n  got  {got1:?}\n  want {expected:?}");
            }
        }
    }
}
