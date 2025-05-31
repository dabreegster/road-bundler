use geo::{Euclidean, InterpolatableLine, Length};

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

    pub fn is_dog_leg(&self, e: EdgeID) -> bool {
        let edge = &self.graph.edges[&e];
        if Euclidean.length(&edge.linestring) > 5.0 {
            return false;
        }
        let mut src_edges = self.graph.intersections[&edge.src].edges.clone();
        let mut dst_edges = self.graph.intersections[&edge.dst].edges.clone();
        if src_edges.len() != 3 || dst_edges.len() != 3 {
            return false;
        }

        // Find the two "side roads" with a different name than the short edge
        // (We could use angle to be safer than name)
        src_edges.retain(|x| self.graph.edges[x].get_name() != edge.get_name());
        dst_edges.retain(|x| self.graph.edges[x].get_name() != edge.get_name());

        if src_edges.len() != 1 || dst_edges.len() != 1 {
            return false;
        }

        let src_edge = &self.graph.edges[&src_edges[0]];
        let dst_edge = &self.graph.edges[&dst_edges[0]];
        // TODO Are these on "different sides" of the short edge?
        // Orient the linestrings to point towards edge.src and edge.dst
        // If the bearings are roughly opposite, then different sides

        true
    }
}
