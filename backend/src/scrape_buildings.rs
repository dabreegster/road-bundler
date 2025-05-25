use std::collections::HashMap;

use geo::{Coord, LineString, Polygon};
use osm_reader::{NodeID, OsmID, RelationID, WayID};
use utils::{osm2graph::OsmReader, Tags};

// TODO Ignores holes
#[derive(Default)]
pub struct OsmBuildings {
    pub polygons: Vec<(WayID, Polygon)>,
}

impl OsmReader for OsmBuildings {
    fn node(&mut self, _: NodeID, _: Coord, _: Tags) {}

    fn way(
        &mut self,
        id: WayID,
        node_ids: &Vec<NodeID>,
        node_mapping: &HashMap<NodeID, Coord>,
        tags: &Tags,
    ) {
        if !tags.has("building") {
            return;
        }

        let exterior = LineString::new(node_ids.into_iter().map(|n| node_mapping[n]).collect());
        self.polygons.push((id, Polygon::new(exterior, Vec::new())));
    }

    fn relation(&mut self, _: RelationID, _: &Vec<(String, OsmID)>, _: &Tags) {}
}
