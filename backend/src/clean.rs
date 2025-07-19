use geo::LineString;

use crate::{EdgeID, IntersectionID, RoadBundler};

impl RoadBundler {
    pub fn remove_edge(&mut self, id: EdgeID) {
        let edge = self.graph.remove_edge(id);
        for i in [edge.src, edge.dst] {
            if self.graph.intersections[&i].edges.is_empty() {
                self.graph.remove_empty_intersection(i);
            }
        }
    }

    pub fn remove_all_service_roads(&mut self) {
        let remove_edges: Vec<_> = self
            .graph
            .edges
            .iter()
            .filter(|(_, edge)| edge.is_service_road(&self.graph))
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

    pub fn collapse_degenerate_intersection(&mut self, id: IntersectionID) {
        let edges = self.graph.intersections[&id].edges.clone();
        // Silently do nothing?
        // TODO in city of london, collapsing all creates a new case of a self-loop somewhere
        if edges.len() != 2 || edges[0] == edges[1] {
            return;
        }

        let mut edge1 = self.graph.remove_edge(edges[0]);
        let mut edge2 = self.graph.remove_edge(edges[1]);
        self.graph.remove_empty_intersection(id);

        // Make edge1 point to id
        let mut pts = Vec::new();
        let i1 = if edge1.src == id {
            edge1.linestring.0.reverse();
            edge1.dst
        } else {
            edge1.src
        };
        pts.extend(edge1.linestring.0);

        // Make edge2 point away from id
        let i2 = if edge2.src == id {
            edge2.dst
        } else {
            edge2.linestring.0.reverse();
            edge2.src
        };
        pts.extend(edge2.linestring.0);

        let e = self.graph.create_new_edge(LineString::new(pts), i1, i2);
        let new_edge = self.graph.edges.get_mut(&e).unwrap();
        new_edge
            .associated_original_edges
            .extend(edge1.associated_original_edges);
        new_edge
            .associated_original_edges
            .extend(edge2.associated_original_edges);
    }
}
