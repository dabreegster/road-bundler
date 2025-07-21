use std::collections::{BTreeMap, HashMap};

use geo::{LineString, Point, Polygon};
use osm_reader::{NodeID, WayID};
use serde::Serialize;
use utils::{Mercator, Tags};

use crate::EdgeKind;

#[derive(Clone)]
pub struct Graph {
    pub edges: BTreeMap<EdgeID, Edge>,
    pub intersections: BTreeMap<IntersectionID, Intersection>,
    // All geometry is stored in world-space
    pub mercator: Mercator,
    pub boundary_polygon: Polygon,

    pub original_edges: HashMap<OriginalEdgeID, OriginalEdge>,

    intersection_id_counter: usize,
    edge_id_counter: usize,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize)]
pub struct EdgeID(pub usize);
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize)]
pub struct OriginalEdgeID(pub usize);
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize)]
pub struct IntersectionID(pub usize);

#[derive(Clone)]
pub struct Edge {
    pub id: EdgeID,
    pub src: IntersectionID,
    pub dst: IntersectionID,
    pub linestring: LineString,
    pub kind: EdgeKind,
}

#[derive(Clone, Serialize)]
pub struct OriginalEdge {
    pub way: WayID,
    pub node1: NodeID,
    pub node2: NodeID,
    pub tags: Tags,
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
        let original_edges = osm_graph
            .edges
            .iter()
            .map(|(id, e)| {
                (
                    OriginalEdgeID(id.0),
                    OriginalEdge {
                        way: e.osm_way,
                        node1: e.osm_node1,
                        node2: e.osm_node2,
                        tags: e.osm_tags.clone(),
                    },
                )
            })
            .collect();

        Self {
            edges: osm_graph
                .edges
                .into_iter()
                .map(|(_, e)| {
                    (
                        e.id.into(),
                        Edge {
                            id: e.id.into(),
                            src: e.src.into(),
                            dst: e.dst.into(),
                            linestring: e.linestring,
                            kind: EdgeKind::initially_classify(e.id, &e.osm_tags),
                        },
                    )
                })
                .collect(),
            intersections: osm_graph
                .intersections
                .into_iter()
                .map(|(_, i)| {
                    (
                        i.id.into(),
                        Intersection {
                            id: i.id.into(),
                            edges: i.edges.into_iter().map(|e| e.into()).collect(),
                            point: i.point,
                            provenance: IntersectionProvenance::OSM(i.osm_node),
                        },
                    )
                })
                .collect(),
            mercator: osm_graph.mercator,
            boundary_polygon: osm_graph.boundary_polygon,
            original_edges,

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

    pub fn remove_edge(&mut self, e: EdgeID) -> Edge {
        let edge = self
            .edges
            .remove(&e)
            .expect("can't remove edge that doesn't exist");
        for i in [edge.src, edge.dst] {
            let intersection = self.intersections.get_mut(&i).unwrap();
            intersection.edges.retain(|x| *x != e);
            // If edge.src == edge.dst, this is idempotent
        }
        edge
    }

    pub fn remove_empty_intersection(&mut self, i: IntersectionID) {
        let intersection = self
            .intersections
            .remove(&i)
            .expect("can't remove intersection that doesn't exist");
        if !intersection.edges.is_empty() {
            panic!("intersection wasn't empty, but removed");
        }
    }

    pub fn remove_all_empty_intersections(&mut self) {
        let remove_intersections: Vec<_> = self
            .intersections
            .iter()
            .filter(|(_, i)| i.edges.is_empty())
            .map(|(id, _)| *id)
            .collect();
        for i in remove_intersections {
            self.remove_empty_intersection(i);
        }
    }

    /// Trusts the linestring to go from `src` to `dst`
    pub fn create_new_edge(
        &mut self,
        linestring: LineString,
        src: IntersectionID,
        dst: IntersectionID,
        kind: EdgeKind,
    ) -> EdgeID {
        let id = self.new_edge_id();
        self.edges.insert(
            id,
            Edge {
                id,
                src,
                dst,
                linestring,
                kind,
            },
        );
        self.intersections.get_mut(&src).unwrap().edges.push(id);
        self.intersections.get_mut(&dst).unwrap().edges.push(id);
        id
    }

    /// Extends the edge geometry in a way that probably overlaps
    pub fn replace_intersection(
        &mut self,
        remove_i: IntersectionID,
        new_intersection: IntersectionID,
        extend_geometry: bool,
    ) {
        let intersection = self
            .intersections
            .remove(&remove_i)
            .expect("can't remove intersection that doesn't exist");
        for surviving_edge in intersection.edges {
            let edge = self.edges.get_mut(&surviving_edge).unwrap();
            let mut updated = false;
            if edge.src == remove_i {
                edge.src = new_intersection;
                // TODO Update IntersectionProvenance?
                if extend_geometry {
                    edge.linestring
                        .0
                        .insert(0, self.intersections[&new_intersection].point.into());
                }
                updated = true;
            }
            if edge.dst == remove_i {
                edge.dst = new_intersection;
                if extend_geometry {
                    edge.linestring
                        .0
                        .push(self.intersections[&new_intersection].point.into());
                }
                updated = true;
            }

            if updated {
                self.intersections
                    .get_mut(&new_intersection)
                    .unwrap()
                    .edges
                    .push(surviving_edge);
            } else {
                panic!("replace_intersection saw inconsistent state about an edge connected to an intersection");
            }
        }
    }
}

// osm2graph's equivalents aren't serializable
impl From<utils::osm2graph::EdgeID> for EdgeID {
    fn from(id: utils::osm2graph::EdgeID) -> Self {
        Self(id.0)
    }
}

impl From<utils::osm2graph::IntersectionID> for IntersectionID {
    fn from(id: utils::osm2graph::IntersectionID) -> Self {
        Self(id.0)
    }
}
