use geo::{Euclidean, InterpolatableLine};

use crate::{EdgeID, Intersection, IntersectionProvenance, RoadBundler};

impl RoadBundler {
    pub fn collapse_edge(&mut self, e: EdgeID) {
        let (src, dst, midpt) = {
            let edge = &self.graph.edges[&e];
            let midpt = edge
                .linestring
                .point_at_ratio_from_start(&Euclidean, 0.5)
                .unwrap();
            (edge.src, edge.dst, midpt)
        };

        self.graph.remove_edge(e);

        // Create a new intersection at the middle of the short edge
        let new_intersection = self.graph.new_intersection_id();
        self.graph.intersections.insert(
            new_intersection,
            Intersection {
                id: new_intersection,
                edges: Vec::new(),
                point: midpt,
                provenance: IntersectionProvenance::Synthetic,
            },
        );

        // Remove the two old intersections, reconnecting the edges and making the linestrings
        // overlap at the ends
        self.graph.replace_intersection(src, new_intersection);
        self.graph.replace_intersection(dst, new_intersection);
    }
}
