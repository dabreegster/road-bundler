use crate::{FaceID, RoadBundler};

impl RoadBundler {
    pub fn merge_sidepath(&mut self, id: FaceID) {
        let face = &self.faces[&id];

        for e in &face.boundary_edges {
            if self.graph.edges[e].is_sidewalk_or_cycleway() {
                self.graph.remove_edge(*e);
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
