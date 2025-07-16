use anyhow::Result;
use geojson::GeoJson;

use crate::geo_helpers::linestring_bearing;
use crate::{Debugger, EdgeID, Face, FaceKind, Graph, RoadBundler};

struct Sidepath {
    sidepath_edges: Vec<EdgeID>,
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
        let mut road_edges = Vec::new();
        let mut sidepath_bearings = Vec::new();
        for e in &face.boundary_edges {
            let edge = &graph.edges[e];
            if edge.is_sidewalk_or_cycleway() {
                if !edge.is_crossing() {
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
    for e in &sidepath.road_edges {
        debug_hover.line(&graph.edges[e].linestring, "parallel road", "blue", 5, 1.0);
    }

    Ok(debug_hover.build())
}

impl RoadBundler {
    pub fn remove_all_sidepaths(&mut self) {
        // Make one pass using the faces, to remember associations
        for face in self.faces.values() {
            if let Ok(info) = Sidepath::maybe_new(&self.graph, face) {
                // TODO This could potentially be many-to-many as expressed now. That seems
                // possible and valid; we need to properly split and match things up.
                for e in info.road_edges {
                    // TODO Make sure all of the roads are original?
                    self.graph
                        .edges
                        .get_mut(&e)
                        .unwrap()
                        .associated_original_edges
                        .extend(info.sidepath_edges.clone());
                }
            }
        }

        let remove_edges: Vec<_> = self
            .graph
            .edges
            .iter()
            .filter(|(_, edge)| edge.is_sidewalk_or_cycleway())
            .map(|(id, _)| *id)
            .collect();
        for e in remove_edges {
            self.graph.remove_edge(e);
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
