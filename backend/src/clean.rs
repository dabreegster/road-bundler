use crate::RoadBundler;

impl RoadBundler {
    pub fn remove_all_service_roads(&mut self) {
        let remove_edges: Vec<_> = self
            .graph
            .edges
            .iter()
            .filter(|(_, edge)| edge.is_service_road())
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
