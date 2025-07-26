use std::collections::HashMap;

use geo::{Centroid, Coord, LineString, Point, Polygon};
use geojson::Feature;
use osm_reader::OsmID;
use rstar::{primitives::GeomWithData, RTree};
use serde::Serialize;
use utils::{osm2graph::OsmReader, Mercator, Tags};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize)]
pub struct AmenityID(pub usize);

#[derive(Clone, Serialize)]
pub struct Amenity {
    pub id: AmenityID,
    pub osm_id: OsmID,
    pub point: Point,
    pub kind: String,
    pub name: Option<String>,
    pub brand: Option<String>,
}

pub struct Amenities {
    pub amenities: Vec<Amenity>,
    pub rtree: RTree<GeomWithData<Point, AmenityID>>,
}

impl ReadAmenities {
    pub fn finalize(mut self, mercator: &Mercator) -> Amenities {
        for a in &mut self.amenities {
            mercator.to_mercator_in_place(&mut a.point);
        }
        let rtree = RTree::bulk_load(
            self.amenities
                .iter()
                .map(|a| GeomWithData::new(a.point, a.id))
                .collect(),
        );
        Amenities {
            amenities: self.amenities,
            rtree,
        }
    }
}

#[derive(Default)]
pub struct ReadAmenities {
    amenities: Vec<Amenity>,
}

impl OsmReader for ReadAmenities {
    fn node(&mut self, id: osm_reader::NodeID, pt: Coord, tags: Tags) {
        if let Some(kind) = Amenity::is_amenity(&tags) {
            self.amenities.push(Amenity::new(
                kind,
                &tags,
                OsmID::Node(id),
                pt.into(),
                AmenityID(self.amenities.len()),
            ));
        }
    }

    fn way(
        &mut self,
        id: osm_reader::WayID,
        node_ids: &Vec<osm_reader::NodeID>,
        node_mapping: &HashMap<osm_reader::NodeID, Coord>,
        tags: &Tags,
    ) {
        if let Some(kind) = Amenity::is_amenity(&tags) {
            let exterior = LineString::new(node_ids.into_iter().map(|n| node_mapping[n]).collect());
            let pt = Polygon::new(exterior, Vec::new()).centroid().unwrap();
            self.amenities.push(Amenity::new(
                kind,
                tags,
                OsmID::Way(id),
                pt,
                AmenityID(self.amenities.len()),
            ));
        }
    }

    // TODO Are there amenities as relations?
    fn relation(
        &mut self,
        _id: osm_reader::RelationID,
        _members: &Vec<(String, OsmID)>,
        _tags: &Tags,
    ) {
    }
}

impl Amenity {
    pub fn new(kind: String, tags: &Tags, osm_id: OsmID, point: Point, id: AmenityID) -> Self {
        Self {
            id,
            osm_id,
            point,
            name: tags.get("name").cloned(),
            kind,
            brand: tags.get("brand").cloned(),
        }
    }

    pub fn to_gj(&self, mercator: &Mercator) -> Feature {
        let mut f = mercator.to_wgs84_gj(&self.point);
        f.set_property("amenity_kind", self.kind.clone());
        f.set_property("osm_id", self.osm_id.to_string());
        if let Some(ref name) = self.name {
            f.set_property("name", name.clone());
        }
        if let Some(ref brand) = self.brand {
            f.set_property("brand", brand.clone());
        }
        f
    }

    fn is_amenity(tags: &Tags) -> Option<String> {
        // Allow everything for now
        tags.get("amenity").or_else(|| tags.get("shop")).cloned()
    }
}
