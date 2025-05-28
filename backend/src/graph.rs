use std::collections::BTreeMap;

use geo::{LineString, Point, Polygon};
use osm_reader::{NodeID, WayID};
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

    pub osm_way: WayID,
    pub osm_node1: NodeID,
    pub osm_node2: NodeID,
    pub osm_tags: Tags,

    pub linestring: LineString,
}

#[derive(Clone)]
pub struct Intersection {
    pub id: IntersectionID,
    pub edges: Vec<EdgeID>,

    pub osm_node: NodeID,

    pub point: Point,
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
                            osm_way: e.osm_way,
                            osm_node1: e.osm_node1,
                            osm_node2: e.osm_node2,
                            osm_tags: e.osm_tags,
                            linestring: e.linestring,
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
                            osm_node: i.osm_node,
                            point: i.point,
                        },
                    )
                })
                .collect(),
            mercator: osm_graph.mercator,
            boundary_polygon: osm_graph.boundary_polygon,
        }
    }
}
