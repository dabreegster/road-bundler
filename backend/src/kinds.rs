use std::collections::BTreeSet;

use serde::Serialize;
use utils::Tags;

use crate::{graph::OriginalEdgeID, Graph};

#[derive(Clone, Serialize)]
pub enum EdgeKind {
    Motorized {
        /// The main driveable roads, possibly in different directions for a dual carriageway.
        /// Unique per edge.
        roads: BTreeSet<OriginalEdgeID>,
        /// Smaller service roads associated. Could include a whole sub-network of service roads
        /// nearby, not just little driveways. So two separate Motorized edges might both reference
        /// the same service roads.
        service_roads: BTreeSet<OriginalEdgeID>,
        /// Footways and cycleways that are parallel to the main driveable road. Might match to
        /// multiple edges.
        sidepaths: BTreeSet<OriginalEdgeID>,
        /// Footway and cycleway crossings and related pieces that aren't parallel to the main
        /// driveable road
        // TODO and maybe pieces of DCs too?
        connectors: BTreeSet<OriginalEdgeID>,
    },
    /// Footways and cycleways that're off-road / not parallel to a driveable road
    Nonmotorized(BTreeSet<OriginalEdgeID>),
}

impl EdgeKind {
    pub fn initially_classify(e: utils::osm2graph::EdgeID, tags: &Tags) -> Self {
        let id = BTreeSet::from([OriginalEdgeID(e.0)]);

        if tags.is_any(
            "highway",
            vec![
                "footway",
                "cycleway",
                "elevator",
                "path",
                "pedestrian",
                "platform",
                "steps",
                "track",
            ],
        ) {
            // These might not be off-road, but we don't know yet
            return Self::Nonmotorized(id);
        }

        if tags.is_any("highway", vec!["corridor", "service"]) {
            return Self::Motorized {
                roads: BTreeSet::new(),
                service_roads: id,
                sidepaths: BTreeSet::new(),
                connectors: BTreeSet::new(),
            };
        }

        Self::Motorized {
            roads: id,
            service_roads: BTreeSet::new(),
            sidepaths: BTreeSet::new(),
            connectors: BTreeSet::new(),
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
                roads: roads1.union(roads2).cloned().collect(),
                service_roads: service_roads1.union(service_roads2).cloned().collect(),
                sidepaths: sidepaths1.union(sidepaths2).cloned().collect(),
                connectors: connectors1.union(connectors2).cloned().collect(),
            }),
            (Self::Nonmotorized(edges1), Self::Nonmotorized(edges2)) => {
                Some(Self::Nonmotorized(edges1.union(edges2).cloned().collect()))
            }
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

    pub fn is_oneway_road(&self, graph: &Graph) -> bool {
        match self {
            EdgeKind::Motorized { roads, .. } => roads
                .iter()
                .all(|e| graph.original_edges[e].tags.is("oneway", "yes")),
            _ => false,
        }
    }

    pub fn is_parking_aisle(&self, graph: &Graph) -> bool {
        match self {
            EdgeKind::Motorized { service_roads, .. } => service_roads
                .iter()
                .all(|e| graph.original_edges[e].tags.is("service", "parking_aisle")),
            _ => false,
        }
    }

    pub fn is_service_road(&self) -> bool {
        match self {
            EdgeKind::Motorized {
                roads,
                service_roads,
                ..
            } => !service_roads.is_empty() && roads.is_empty(),
            _ => false,
        }
    }

    /// Only if it's the same for all constituents
    pub fn get_road_name<'a>(&self, graph: &'a Graph) -> Option<&'a String> {
        match self {
            EdgeKind::Motorized { roads, .. } => {
                let names: BTreeSet<_> = roads
                    .iter()
                    .map(|e| graph.original_edges[e].tags.get("name"))
                    .collect();
                if names.len() != 1 {
                    return None;
                }
                names.into_iter().next().unwrap()
            }
            _ => None,
        }
    }
}
