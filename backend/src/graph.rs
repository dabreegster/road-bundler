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
        }
    }
}
