use anyhow::{Context, Result};
use geo::{Distance, Euclidean, LineString};
use geojson::GeoJson;
use itertools::Itertools;
use serde::Serialize;

use crate::geo_helpers::{
    average_linestrings, collapse_degree_2, linestring_bearing, KeyedLineString,
};
use crate::split_line::Splits;
use crate::{Debugger, EdgeID, Face, FaceID, FaceKind, Graph, RoadBundler};

// TODO Don't serialize this. Plumb the extra debug info as foreign members?
#[derive(Serialize)]
pub struct DualCarriageway {
    pub name: String,
    pub center_line: LineString,
    #[serde(skip)]
    pub splits: Splits,

    pub debug_hover: GeoJson,
}

impl DualCarriageway {
    pub fn maybe_new(graph: &Graph, face: &Face) -> Result<Self> {
        let (name, dc_edges) = detect_dc_edges(graph, face)?;

        let mut edge_bearings: Vec<(EdgeID, f64)> = dc_edges
            .into_iter()
            .map(|e| (e, linestring_bearing(&graph.edges[&e].linestring)))
            .collect();
        edge_bearings.sort_by_key(|(_, x)| (*x * 10e5) as usize);
        let bearings: Vec<f64> = edge_bearings.iter().map(|(_, x)| *x).collect();
        let classes = classify_bearings(bearings);

        let mut side1 = Vec::new();
        let mut side2 = Vec::new();
        for ((e, _), class) in edge_bearings.into_iter().zip(classes.into_iter()) {
            if class == 0 {
                side1.push(e);
            } else {
                side2.push(e);
            }
        }

        let side1_joined = collapse_degree_2(
            side1
                .iter()
                .map(|e| KeyedLineString {
                    linestring: graph.edges[e].linestring.clone(),
                    ids: vec![(*e, true)],
                })
                .collect(),
        );
        let side2_joined = collapse_degree_2(
            side2
                .iter()
                .map(|e| KeyedLineString {
                    linestring: graph.edges[e].linestring.clone(),
                    ids: vec![(*e, true)],
                })
                .collect(),
        );
        if side1_joined.len() != 1 || side2_joined.len() != 1 {
            bail!(
                "Not a DC because we have {} and {} joined line results",
                side1_joined.len(),
                side2_joined.len()
            );
        }

        let center_line =
            average_linestrings(&side1_joined[0].linestring, &side2_joined[0].linestring)?;
        let splits = crate::split_line::split_center(graph, &center_line, face);

        let mut debug_hover = Debugger::new(graph.mercator.clone());
        for e in &side1 {
            debug_hover.line(&graph.edges[e].linestring, "side1 edge", "purple", 5, 1.0);
        }
        for e in &side2 {
            debug_hover.line(&graph.edges[e].linestring, "side2 edge", "blue", 5, 1.0);
        }
        debug_hover.line(&side1_joined[0].linestring, "side1 full", "purple", 15, 0.5);
        debug_hover.line(&side2_joined[0].linestring, "side2 full", "blue", 15, 0.5);
        debug_hover.line(&center_line, "new center", "black", 10, 1.0);
        for e in &face.boundary_edges {
            if !side1.contains(e) && !side2.contains(e) {
                debug_hover.line(
                    &graph.edges[e].linestring,
                    "leftover boundary edge",
                    "red",
                    5,
                    1.0,
                );
            }
        }
        for e in &face.connecting_edges {
            debug_hover.line(
                &graph.edges[e].linestring,
                "connecting edge",
                "yellow",
                5,
                1.0,
            );
        }
        for pt in &splits.new_endpts {
            debug_hover.circle(*pt, "split", "green", 5);
        }

        Ok(Self {
            name,
            center_line,
            splits,
            debug_hover: debug_hover.build(),
        })
    }
}

fn detect_dc_edges(graph: &Graph, face: &Face) -> Result<(String, Vec<EdgeID>)> {
    if face.kind != FaceKind::RoadArtifact {
        bail!("Face isn't a road artifact");
    }

    // Find all of the oneway edges with a name
    let oneways: Vec<EdgeID> = face
        .boundary_edges
        .iter()
        .filter(|e| graph.edges[e].is_oneway() && graph.edges[e].get_name().is_some())
        .cloned()
        .collect();

    // Group by their name
    let oneways_by_name = oneways
        .into_iter()
        .into_group_map_by(|e| graph.edges[e].get_name().unwrap());

    // Pick the group with the most members
    let (name, dc_edges) = oneways_by_name
        .into_iter()
        .max_by_key(|(_, list)| list.len())
        .context("no oneways")?;

    // Make sure we have at least two edges
    if dc_edges.len() < 2 {
        bail!("not enough edges to form a DC");
    }

    Ok((name.to_string(), dc_edges))
}

// Assumes input is sorted
fn classify_bearings(bearings: Vec<f64>) -> Vec<usize> {
    let mut classes = Vec::new();

    let mut last_class = 0;
    let mut last_bearing = 0.0;
    for bearing in bearings {
        if classes.is_empty() {
            last_bearing = bearing;
            classes.push(0);
            continue;
        }

        // Simple heuristic that passes all tests so far, and handles wraparound cases
        if (bearing - last_bearing).abs() > 45.0 {
            last_class = if last_class == 0 { 1 } else { 0 };
        }
        last_bearing = bearing;
        classes.push(last_class);
    }

    classes
}

impl RoadBundler {
    pub fn collapse_dual_carriageway(&mut self, id: FaceID) {
        let face = &self.faces[&id];
        let dc = crate::dual_carriageway::DualCarriageway::maybe_new(&self.graph, face)
            .expect("collapse_dual_carriageway on something that isn't a DC");

        // Remove all the boundary_edges
        let mut associated_original_edges = Vec::new();
        for e in &face.boundary_edges {
            let edge = self.graph.remove_edge(*e);
            associated_original_edges.push(*e);
            associated_original_edges.extend(edge.associated_original_edges);
        }

        // Create the new split center-lines, with new intersections
        // TODO Which of the original edges should be associated_original_edges? For now, all
        let new_intersections = self.graph.create_new_linked_edges(
            dc.splits.lines,
            dc.splits.new_endpts,
            associated_original_edges,
        );

        // Re-attach every connecting edge to the nearest new intersection
        // (we could maybe preserve more info to do this directly?)
        for e in &face.connecting_edges {
            let edge = &self.graph.edges[e];

            // There could be a loop; handle each endpoint if needed
            for existing_i in [edge.src, edge.dst] {
                if !face.boundary_intersections.contains(&existing_i) {
                    continue;
                }
                let existing_pt = self.graph.intersections[&existing_i].point;

                let closest_new_i = *new_intersections
                    .iter()
                    .min_by_key(|i| {
                        (10e6 * Euclidean.distance(existing_pt, self.graph.intersections[i].point))
                            as usize
                    })
                    .unwrap();
                self.graph.create_new_edge(
                    LineString::new(vec![
                        existing_pt.into(),
                        self.graph.intersections[&closest_new_i].point.into(),
                    ]),
                    existing_i,
                    closest_new_i,
                );
            }
        }

        // Remove orphaned intersections
        for i in &face.boundary_intersections {
            if self.graph.intersections[i].edges.is_empty() {
                self.graph.remove_empty_intersection(*i);
            }
        }
    }
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
            let got1 = classify_bearings(input.iter().map(|b| *b as f64).collect());
            // Classes 0 and 1 are arbitrary; the opposite is also fine
            let got2: Vec<usize> = got1.iter().map(|c| if *c == 0 { 1 } else { 0 }).collect();
            if got1 != expected && got2 != expected {
                panic!("For bearings {input:?},\n  got  {got1:?}\n  want {expected:?}");
            }
        }
    }
}
