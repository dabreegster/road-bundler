use std::collections::{BTreeMap, BTreeSet, HashMap};

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
    pub tags_per_way: HashMap<WayID, Tags>,

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

    /// Any edges from the original_graph that've been consolidated into this one. It should maybe
    /// only be defined for synthetic edges, but sidepath matching is still TBD
    pub associated_original_edges: BTreeSet<EdgeID>,
}

impl Edge {
    pub fn is_oneway(&self, graph: &Graph) -> bool {
        match self.provenance {
            EdgeProvenance::OSM { way, .. } => graph.tags_per_way[&way].is("oneway", "yes"),
            EdgeProvenance::Synthetic => false,
        }
    }

    pub fn is_parking_aisle(&self, graph: &Graph) -> bool {
        match self.provenance {
            EdgeProvenance::OSM { way, .. } => {
                graph.tags_per_way[&way].is("service", "parking_aisle")
            }
            EdgeProvenance::Synthetic => false,
        }
    }

    pub fn is_service_road(&self, graph: &Graph) -> bool {
        match self.provenance {
            EdgeProvenance::OSM { way, .. } => {
                graph.tags_per_way[&way].is_any("highway", vec!["corridor", "service"])
            }
            EdgeProvenance::Synthetic => false,
        }
    }

    // TODO Rename and handle more cases
    pub fn is_sidewalk_or_cycleway(&self, graph: &Graph) -> bool {
        match self.provenance {
            EdgeProvenance::OSM { way, .. } => graph.tags_per_way[&way].is_any(
                "highway",
                vec![
                    "footway", "cycleway", "elevator", "path", "platform", "steps", "track",
                ],
            ),
            EdgeProvenance::Synthetic => false,
        }
    }

    pub fn is_crossing(&self, graph: &Graph) -> bool {
        match self.provenance {
            EdgeProvenance::OSM { way, .. } => {
                graph.tags_per_way[&way].is_any("footway", vec!["crossing", "traffic_island"])
                    || graph.tags_per_way[&way].is("cycleway", "crossing")
            }
            EdgeProvenance::Synthetic => false,
        }
    }

    pub fn get_name<'a>(&self, graph: &'a Graph) -> Option<&'a String> {
        match self.provenance {
            EdgeProvenance::OSM { way, .. } => graph.tags_per_way[&way].get("name"),
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
        let tags_per_way = osm_graph
            .edges
            .values()
            .map(|e| (e.osm_way, e.osm_tags.clone()))
            .collect();

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
                            },
                            associated_original_edges: BTreeSet::new(),
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
            tags_per_way,

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

    /// Returns the new intersections created
    pub fn create_new_linked_edges(
        &mut self,
        linestrings: Vec<LineString>,
        endpoints: Vec<Point>,
        associated_original_edges: Vec<EdgeID>,
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
            let e = self.create_new_edge(
                linestring,
                new_intersections[idx],
                new_intersections[idx + 1],
            );
            // TODO For now, all match up
            self.edges
                .get_mut(&e)
                .unwrap()
                .associated_original_edges
                .extend(associated_original_edges.clone());
        }

        new_intersections
    }

    /// Trusts the linestring to go from `src` to `dst`
    pub fn create_new_edge(
        &mut self,
        linestring: LineString,
        src: IntersectionID,
        dst: IntersectionID,
    ) -> EdgeID {
        let id = self.new_edge_id();
        self.edges.insert(
            id,
            Edge {
                id,
                src,
                dst,
                linestring,
                provenance: EdgeProvenance::Synthetic,
                associated_original_edges: BTreeSet::new(),
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
                // TODO In provenance, should we mark modified cases?
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
