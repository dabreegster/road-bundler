use std::collections::HashMap;

use geo::{Coord, LineString, Polygon};
use osm_reader::{NodeID, OsmID, RelationID, WayID};
use utils::{osm2graph::OsmReader, Tags};

// TODO Ignores holes
#[derive(Default)]
pub struct OsmBuildings {
    // TODO Maybe OsmID instead, to cover relations
    pub polygons: Vec<(WayID, Polygon)>,

    possible_building_parts: HashMap<WayID, Polygon>,
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
        if tags.0.is_empty() && node_ids[0] == *node_ids.last().unwrap() {
            self.possible_building_parts.insert(
                id,
                Polygon::new(
                    LineString::new(node_ids.into_iter().map(|n| node_mapping[n]).collect()),
                    Vec::new(),
                ),
            );
            return;
        }

        if !tags.has("building") {
            return;
        }

        let exterior = LineString::new(node_ids.into_iter().map(|n| node_mapping[n]).collect());
        self.polygons.push((id, Polygon::new(exterior, Vec::new())));
    }

    fn relation(&mut self, _: RelationID, members: &Vec<(String, OsmID)>, tags: &Tags) {
        if !tags.has("building") {
            return;
        }
        for (role, id) in members {
            // TODO if-let
            if role != "outer" {
                continue;
            }
            if let OsmID::Way(way) = id {
                if let Some(polygon) = self.possible_building_parts.remove(&way) {
                    self.polygons.push((*way, polygon));
                }
            }
        }
    }
}
