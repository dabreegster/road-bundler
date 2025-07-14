use std::collections::HashMap;

use geo::{Coord, LineString, Polygon};
use osm_reader::{NodeID, OsmID, RelationID, WayID};
use utils::{osm2graph::OsmReader, Tags};

// TODO Ignores holes
#[derive(Default)]
pub struct OsmAreas {
    // TODO Maybe OsmID to capture relations fully
    pub polygons: Vec<(WayID, AreaKind, Polygon)>,
    possible_area_parts: HashMap<WayID, Polygon>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AreaKind {
    Building,
    Water,
    Park,
}

impl AreaKind {
    fn from_tags(tags: &Tags) -> Option<Self> {
        if tags.has("building") {
            Some(Self::Building)
        } else if tags.is("leisure", "park") {
            Some(Self::Park)
        } else if tags.is("natural", "water") {
            Some(Self::Water)
        } else {
            None
        }
    }
}

impl OsmReader for OsmAreas {
    fn node(&mut self, _: NodeID, _: Coord, _: Tags) {}

    fn way(
        &mut self,
        id: WayID,
        node_ids: &Vec<NodeID>,
        node_mapping: &HashMap<NodeID, Coord>,
        tags: &Tags,
    ) {
        if tags.0.is_empty() && node_ids[0] == *node_ids.last().unwrap() {
            self.possible_area_parts.insert(
                id,
                Polygon::new(
                    LineString::new(node_ids.into_iter().map(|n| node_mapping[n]).collect()),
                    Vec::new(),
                ),
            );
            return;
        }

        if let Some(kind) = AreaKind::from_tags(tags) {
            let exterior = LineString::new(node_ids.into_iter().map(|n| node_mapping[n]).collect());
            self.polygons
                .push((id, kind, Polygon::new(exterior, Vec::new())));
        }
    }

    fn relation(&mut self, _: RelationID, members: &Vec<(String, OsmID)>, tags: &Tags) {
        let Some(kind) = AreaKind::from_tags(tags) else {
            return;
        };
        for (role, id) in members {
            // TODO if-let
            if role != "outer" {
                continue;
            }
            if let OsmID::Way(way) = id {
                if let Some(polygon) = self.possible_area_parts.remove(&way) {
                    self.polygons.push((*way, kind, polygon));
                }
            }
        }
    }
}
