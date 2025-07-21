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
            .filter(|(_, edge)| edge.kind.is_service_road())
            .map(|(id, _)| *id)
            .collect();
        for e in remove_edges {
            self.graph.remove_edge(e);
        }

        self.graph.remove_all_empty_intersections();
    }

    pub fn collapse_degenerate_intersection(&mut self, id: IntersectionID) {
        let edges = self.graph.intersections[&id].edges.clone();
        // Silently do nothing?
        // TODO in city of london, collapsing all creates a new case of a self-loop somewhere
        if edges.len() != 2 || edges[0] == edges[1] {
            return;
        }

        // Can't combine a motorized and nonmotorized edge. Again, maybe weird to silently do
        // nothing.
        let Some(kind) = self.graph.edges[&edges[0]]
            .kind
            .merge(&self.graph.edges[&edges[1]].kind)
        else {
            return;
        };

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

        self.graph
            .create_new_edge(LineString::new(pts), i1, i2, kind);
    }
}
