use std::collections::{BTreeMap, BTreeSet, HashMap};

use geo::{LineString, Point, Polygon};
use osm_reader::{NodeID, WayID};
use serde::Serialize;
use utils::{Mercator, Tags};

#[derive(Clone)]
pub struct Graph {
    pub edges: BTreeMap<EdgeID, Edge>,
    pub intersections: BTreeMap<IntersectionID, Intersection>,
    // All geometry is stored in world-space
    pub mercator: Mercator,
    pub boundary_polygon: Polygon,

    pub original_edges: HashMap<OriginalEdgeID, OriginalEdge>,
    // TODO Get rid of this one
    pub tags_per_way: HashMap<WayID, Tags>,

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
    // TODO remove
    pub provenance: EdgeProvenance,
    pub kind: EdgeKind,

    /// Any edges from the original_graph that've been consolidated into this one. It should maybe
    /// only be defined for synthetic edges, but sidepath matching is still TBD
    // TODO remove
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

#[derive(Clone, Serialize)]
pub struct OriginalEdge {
    pub way: WayID,
    pub node1: NodeID,
    pub node2: NodeID,
    pub tags: Tags,
}

#[derive(Clone, Serialize)]
pub enum EdgeKind {
    Motorized {
        /// The main driveable roads, possibly in different directions for a dual carriageway.
        /// Unique per edge.
        roads: Vec<OriginalEdgeID>,
        /// Smaller service roads associated. Could include a whole sub-network of service roads
        /// nearby, not just little driveways. So two separate Motorized edges might both reference
        /// the same service roads.
        service_roads: Vec<OriginalEdgeID>,
        /// Footways and cycleways that are parallel to the main driveable road. Might match to
        /// multiple edges.
        sidepaths: Vec<OriginalEdgeID>,
        /// Footway and cycleway crossings and related pieces that aren't parallel to the main
        /// driveable road
        // TODO and maybe pieces of DCs too?
        connectors: Vec<OriginalEdgeID>,
    },
    /// Footways and cycleways that're off-road / not parallel to a driveable road
    Nonmotorized(Vec<OriginalEdgeID>),
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
                            provenance: EdgeProvenance::OSM {
                                way: e.osm_way,
                                node1: e.osm_node1,
                                node2: e.osm_node2,
                            },
                            associated_original_edges: BTreeSet::new(),
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
            tags_per_way,
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

impl EdgeKind {
    fn initially_classify(e: utils::osm2graph::EdgeID, tags: &Tags) -> Self {
        let id = vec![OriginalEdgeID(e.0)];

        if tags.is_any(
            "highway",
            vec![
                "footway", "cycleway", "elevator", "path", "platform", "steps", "track",
            ],
        ) {
            // These might not be off-road, but we don't know yet
            return Self::Nonmotorized(id);
        }

        if tags.is_any("highway", vec!["corridor", "service"]) {
            return Self::Motorized {
                roads: Vec::new(),
                service_roads: id,
                sidepaths: Vec::new(),
                connectors: Vec::new(),
            };
        }

        Self::Motorized {
            roads: id,
            service_roads: Vec::new(),
            sidepaths: Vec::new(),
            connectors: Vec::new(),
        }
    }

    /// Only two Motorized or two Nonmotorized edges can be combined
    pub fn merge(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (
                Self::Motorized {
                    roads: roads1,
                    service_roads: service_roads1,
                    sidepaths: sidepaths1,
                    connectors: connectors1,
                },
                Self::Motorized {
                    roads: roads2,
                    service_roads: service_roads2,
                    sidepaths: sidepaths2,
                    connectors: connectors2,
                },
            ) => Some(Self::Motorized {
                roads: roads1.iter().chain(roads2).cloned().collect(),
                // TODO Should dedupe to be careful
                service_roads: service_roads1
                    .iter()
                    .chain(service_roads2)
                    .cloned()
                    .collect(),
                sidepaths: sidepaths1.iter().chain(sidepaths2).cloned().collect(),
                connectors: connectors1.iter().chain(connectors2).cloned().collect(),
            }),
            (Self::Nonmotorized(edges1), Self::Nonmotorized(edges2)) => Some(Self::Nonmotorized(
                edges1.iter().chain(edges2).cloned().collect(),
            )),
            _ => None,
        }
    }

    pub fn to_simple(&self) -> &'static str {
        match self {
            Self::Motorized {
                roads,
                service_roads,
                sidepaths,
                ..
            } => {
                if !roads.is_empty() {
                    return "road";
                }
                if !service_roads.is_empty() {
                    return "service road";
                }
                if !sidepaths.is_empty() {
                    return "sidepath";
                }
                // TODO Normally one should be non-empty, but use this as a fallback for now
                "connector"
            }
            Self::Nonmotorized(_) => "nonmotorized",
        }
    }
}
