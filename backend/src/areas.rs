use std::collections::HashMap;

use geo::{Centroid, Coord, LineString, Point, Polygon};
use osm_reader::{NodeID, OsmID, RelationID, WayID};
use rstar::RTree;
use utils::{osm2graph::OsmReader, Mercator, Tags};

pub struct Areas {
    pub building_polygons: RTree<Polygon>,
    pub building_centroids: RTree<Point>,

    pub other_polygons: RTree<Polygon>,
    pub other_centroids: RTree<Point>,
}

// TODO Ignores holes
#[derive(Default)]
pub struct ReadOsmAreas {
    // TODO Maybe OsmID to capture relations fully
    polygons: Vec<(WayID, AreaKind, Polygon)>,
    possible_area_parts: HashMap<WayID, Polygon>,
}

#[derive(Clone, Copy, PartialEq)]
enum AreaKind {
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

impl OsmReader for ReadOsmAreas {
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

impl ReadOsmAreas {
    pub fn finalize(self, mercator: &Mercator) -> Areas {
        let mut building_polygons = Vec::new();
        let mut building_centroids = Vec::new();
        let mut other_polygons = Vec::new();
        let mut other_centroids = Vec::new();

        for (_, kind, mut polygon) in self.polygons {
            mercator.to_mercator_in_place(&mut polygon);
            if kind == AreaKind::Building {
                building_centroids.extend(polygon.centroid());
                building_polygons.push(polygon);
            } else {
                other_centroids.extend(polygon.centroid());
                other_polygons.push(polygon);
            }
        }
        let building_polygons = RTree::bulk_load(building_polygons);
        let building_centroids = RTree::bulk_load(building_centroids);
        let other_polygons = RTree::bulk_load(other_polygons);
        let other_centroids = RTree::bulk_load(other_centroids);
        Areas {
            building_polygons,
            building_centroids,
            other_polygons,
            other_centroids,
        }
    }
}
