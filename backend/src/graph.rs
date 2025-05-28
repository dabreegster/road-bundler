use std::collections::BTreeMap;

use geo::{LineString, Point, Polygon};
use osm_reader::{NodeID, WayID};
use serde::Serialize;
pub use utils::osm2graph::{EdgeID, IntersectionID};
use utils::{Mercator, Tags};

#[derive(Clone)]
pub struct Graph {
    pub edges: BTreeMap<EdgeID, Edge>,
    pub intersections: BTreeMap<IntersectionID, Intersection>,
    // All geometry is stored in world-space
    pub mercator: Mercator,
    pub boundary_polygon: Polygon,

    intersection_id_counter: usize,
    edge_id_counter: usize,
}

#[derive(Clone)]
pub struct Edge {
    pub id: EdgeID,
    pub src: IntersectionID,
    pub dst: IntersectionID,
    pub linestring: LineString,
    pub provenance: EdgeProvenance,
}

impl Edge {
    pub fn is_oneway(&self) -> bool {
        match self.provenance {
            EdgeProvenance::OSM { ref tags, .. } => tags.is("oneway", "yes"),
            EdgeProvenance::Synthetic => false,
        }
    }

    pub fn get_name(&self) -> Option<&String> {
        match self.provenance {
            EdgeProvenance::OSM { ref tags, .. } => tags.get("name"),
            EdgeProvenance::Synthetic => None,
        }
    }
}

#[derive(Clone, Serialize)]
pub enum EdgeProvenance {
    OSM {
        way: WayID,
        node1: NodeID,
        node2: NodeID,
        tags: Tags,
    },
    Synthetic,
}

#[derive(Clone)]
pub struct Intersection {
    #[allow(unused)]
    pub id: IntersectionID,
    pub edges: Vec<EdgeID>,
    pub point: Point,
    #[allow(unused)]
    pub provenance: IntersectionProvenance,
}

#[derive(Clone, Serialize)]
pub enum IntersectionProvenance {
    OSM(NodeID),
    Synthetic,
}

impl Graph {
    pub fn new(osm_graph: utils::osm2graph::Graph) -> Self {
        let intersection_id_counter = osm_graph.intersections.keys().max().unwrap().0 + 1;
        let edge_id_counter = osm_graph.edges.keys().max().unwrap().0 + 1;

        Self {
            edges: osm_graph
                .edges
                .into_iter()
                .map(|(_, e)| {
                    (
                        e.id,
                        Edge {
                            id: e.id,
                            src: e.src,
                            dst: e.dst,
                            linestring: e.linestring,
                            provenance: EdgeProvenance::OSM {
                                way: e.osm_way,
                                node1: e.osm_node1,
                                node2: e.osm_node2,
                                tags: e.osm_tags,
                            },
                        },
                    )
                })
                .collect(),
            intersections: osm_graph
                .intersections
                .into_iter()
                .map(|(_, i)| {
                    (
                        i.id,
                        Intersection {
                            id: i.id,
                            edges: i.edges,
                            point: i.point,
                            provenance: IntersectionProvenance::OSM(i.osm_node),
                        },
                    )
                })
                .collect(),
            mercator: osm_graph.mercator,
            boundary_polygon: osm_graph.boundary_polygon,

            intersection_id_counter,
            edge_id_counter,
        }
    }

    pub fn new_intersection_id(&mut self) -> IntersectionID {
        self.intersection_id_counter += 1;
        IntersectionID(self.intersection_id_counter)
    }

    pub fn new_edge_id(&mut self) -> EdgeID {
        self.edge_id_counter += 1;
        EdgeID(self.edge_id_counter)
    }

    pub fn remove_edge(&mut self, e: EdgeID) {
        let edge = self
            .edges
            .remove(&e)
            .expect("can't remove edge that doesn't exist");
        for i in [edge.src, edge.dst] {
            let intersection = self.intersections.get_mut(&i).unwrap();
            intersection.edges.retain(|x| *x != e);
            // If edge.src == edge.dst, this is idempotent
        }
    }

    /// Returns the new intersections created
    pub fn create_new_linked_edges(
        &mut self,
        linestrings: Vec<LineString>,
        endpoints: Vec<Point>,
    ) -> Vec<IntersectionID> {
        // Assumes linestrings all point in the correct way
        // Assumes endpoints comes from linestring_endpoints (TODO maybe just call it here)
        assert_eq!(linestrings.len() + 1, endpoints.len());

        let mut new_intersections = Vec::new();
        for point in endpoints {
            let id = self.new_intersection_id();
            self.intersections.insert(
                id,
                Intersection {
                    id,
                    edges: vec![],
                    point,
                    provenance: IntersectionProvenance::Synthetic,
                },
            );
            new_intersections.push(id);
        }

        for (idx, linestring) in linestrings.into_iter().enumerate() {
            self.create_new_edge(
                linestring,
                new_intersections[idx],
                new_intersections[idx + 1],
            );
        }

        new_intersections
    }

    /// Trusts the linestring to go from `src` to `dst`
    pub fn create_new_edge(
        &mut self,
        linestring: LineString,
        src: IntersectionID,
        dst: IntersectionID,
    ) {
        let id = self.new_edge_id();
        self.edges.insert(
            id,
            Edge {
                id,
                src,
                dst,
                linestring,
                provenance: EdgeProvenance::Synthetic,
            },
        );
        self.intersections.get_mut(&src).unwrap().edges.push(id);
        self.intersections.get_mut(&dst).unwrap().edges.push(id);
    }
}
