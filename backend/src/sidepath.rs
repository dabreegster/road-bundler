use anyhow::Result;
use geojson::GeoJson;

use crate::geo_helpers::linestring_bearing;
use crate::{Debugger, EdgeID, EdgeKind, Face, FaceKind, Graph, RoadBundler};

struct Sidepath {
    sidepath_edges: Vec<EdgeID>,
    connector_edges: Vec<EdgeID>,
    road_edges: Vec<EdgeID>,
}

impl Sidepath {
    fn maybe_new(graph: &Graph, face: &Face) -> Result<Self> {
        if face.kind != FaceKind::SidepathArtifact {
            bail!("not a SidepathArtifact");
        }

        // Find the "main parts" of the sidepath -- not a crossing
        // TODO https://www.openstreetmap.org/way/974152886
        // We have to use angles too -- there's a tiny bit of sidewalk before the crossing
        let mut sidepath_edges = Vec::new();
        let mut connector_edges = Vec::new();
        let mut road_edges = Vec::new();
        let mut sidepath_bearings = Vec::new();
        for e in &face.boundary_edges {
            let edge = &graph.edges[e];
            // We're assuming EdgeType::Nonmotorized for these
            if edge.is_sidewalk_or_cycleway(graph) {
                if edge.is_crossing(graph) {
                    connector_edges.push(*e);
                } else {
                    sidepath_edges.push(*e);
                    sidepath_bearings.push(linestring_bearing(&edge.linestring));
                }
            } else {
                road_edges.push(*e);
            }
        }

        // And then filter the road edges so that they're parallel-ish to some part of the sidepath.
        // We assume the entire road edge has the sidepath. If we need to be more precise later, we'll
        // have to project endpoints and slice the line.
        //
        // TODO Do we need to do similar to turn some sidepath_edges into connector_edges?
        road_edges.retain(|e| {
            let bearing = linestring_bearing(&graph.edges[e].linestring);
            sidepath_bearings
                .iter()
                .any(|b| roughly_parallel(bearing, *b))
        });

        if sidepath_edges.is_empty() || road_edges.is_empty() {
            bail!("sidepath or matching road is missing");
        }

        Ok(Self {
            sidepath_edges,
            connector_edges,
            road_edges,
        })
    }
}

/// Return debug info about a sidepath
pub fn detect_sidepath(graph: &Graph, face: &Face) -> Result<GeoJson> {
    let sidepath = Sidepath::maybe_new(graph, face)?;

    let mut debug_hover = Debugger::new(graph.mercator.clone());
    for e in &sidepath.sidepath_edges {
        debug_hover.line(
            &graph.edges[e].linestring,
            "proper sidepath",
            "purple",
            5,
            1.0,
        );
    }
    for e in &sidepath.connector_edges {
        debug_hover.line(&graph.edges[e].linestring, "connector", "red", 5, 1.0);
    }
    for e in &sidepath.road_edges {
        debug_hover.line(&graph.edges[e].linestring, "parallel road", "blue", 5, 1.0);
    }

    Ok(debug_hover.build())
}

impl RoadBundler {
    pub fn remove_all_sidepaths(&mut self) {
        // Make one pass using the faces, to remember associations
        let mut remove_edges = Vec::new();
        for face in self.faces.values() {
            if let Ok(info) = Sidepath::maybe_new(&self.graph, face) {
                let mut original_sidepaths = Vec::new();
                for e in info.sidepath_edges {
                    remove_edges.push(e);

                    match &self.graph.edges[&e].kind {
                        EdgeKind::Nonmotorized(orig) => {
                            original_sidepaths.extend(orig.clone());
                        }
                        _ => panic!("A sidepath was Motorized"),
                    }
                }

                let mut original_connectors = Vec::new();
                for e in info.connector_edges {
                    remove_edges.push(e);

                    match &self.graph.edges[&e].kind {
                        EdgeKind::Nonmotorized(orig) => {
                            original_connectors.extend(orig.clone());
                        }
                        _ => panic!("A sidepath is Motorized"),
                    }
                }

                // Create a many-to-many relationship -- every road will reference every piece of
                // sidepath and connector. We could try some kind of linear referencing later to
                // clean it up.
                for e in info.road_edges {
                    match self.graph.edges.get_mut(&e).unwrap().kind {
                        EdgeKind::Motorized {
                            ref mut sidepaths,
                            ref mut connectors,
                            ..
                        } => {
                            sidepaths.extend(original_sidepaths.clone());
                            connectors.extend(original_connectors.clone());
                        }
                        _ => panic!("A road edge is Nonmotorized"),
                    }
                }
            }
        }

        for e in remove_edges {
            // TODO Not sure why something is part of two SidepathArtifacts, but don't crash
            if self.graph.edges.contains_key(&e) {
                self.graph.remove_edge(e);
            }
        }

        let remove_intersections: Vec<_> = self
            .graph
            .intersections
            .iter()
            .filter(|(_, i)| i.edges.is_empty())
            .map(|(id, _)| *id)
            .collect();
        for i in remove_intersections {
            self.graph.remove_empty_intersection(i);
        }
    }
}

// or anti-parallel
fn roughly_parallel(b1: f64, b2: f64) -> bool {
    let diff = ((b1 - b2 + 180.0).abs() % 360.0 - 180.0).abs();
    let tolerance = 30.0;
    diff < tolerance || (diff - 180.0).abs() < tolerance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roughly_parallel() {
        assert!(roughly_parallel(359., 360.));
        assert!(roughly_parallel(359., 0.));
        assert!(roughly_parallel(354., 2.));
        assert!(roughly_parallel(179., 359.));

        assert!(!roughly_parallel(179., 271.));
        // south vs west
    }
}
