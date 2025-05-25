#[allow(unused_imports)]
#[macro_use]
extern crate anyhow;
#[allow(unused_imports)]
#[macro_use]
extern crate log;

use std::sync::Once;

use anyhow::Result;
use geo::{Centroid, Point};
use geojson::GeoJson;
use utils::{osm2graph::Graph, Tags};
use wasm_bindgen::prelude::*;

use crate::faces::{make_faces, Face};

mod faces;
mod scrape_buildings;
mod slice_nearest_boundary;

static START: Once = Once::new();

#[wasm_bindgen]
pub struct RoadBundler {
    graph: Graph,
    faces: Vec<Face>,
    building_centroids: Vec<Point>,
}

#[wasm_bindgen]
impl RoadBundler {
    #[wasm_bindgen(constructor)]
    pub fn new(input_bytes: &[u8]) -> Result<RoadBundler, JsValue> {
        // Panics shouldn't happen, but if they do, console.log them.
        console_error_panic_hook::set_once();
        START.call_once(|| {
            console_log::init_with_level(log::Level::Info).unwrap();
        });

        let mut buildings = scrape_buildings::OsmBuildings::default();
        let mut graph = utils::osm2graph::Graph::new(input_bytes, keep_edge, &mut buildings)
            .map_err(err_to_js)?;
        graph.compact_ids();

        let mut building_centroids = Vec::new();
        for (_, polygon) in &mut buildings.polygons {
            graph.mercator.to_mercator_in_place(polygon);
            building_centroids.extend(polygon.centroid());
        }

        let faces = make_faces(&graph, &building_centroids);
        Ok(Self {
            graph,
            faces,
            building_centroids,
        })
    }

    #[wasm_bindgen(js_name = getEdges)]
    pub fn get_edges(&self) -> Result<String, JsValue> {
        let mut features = Vec::new();
        for (id, edge) in &self.graph.edges {
            let mut f = self.graph.mercator.to_wgs84_gj(&edge.linestring);
            f.id = Some(geojson::feature::Id::Number(id.0.into()));
            f.set_property("edge_id", id.0);
            f.set_property("osm_way", edge.osm_way.0);
            f.set_property(
                "osm_tags",
                serde_json::to_value(&edge.osm_tags).map_err(err_to_js)?,
            );
            features.push(f);
        }
        serde_json::to_string(&GeoJson::from(features)).map_err(err_to_js)
    }

    #[wasm_bindgen(js_name = getFaces)]
    pub fn get_faces(&self) -> Result<String, JsValue> {
        let mut features = Vec::new();
        for face in &self.faces {
            let mut f = self.graph.mercator.to_wgs84_gj(&face.polygon);
            f.set_property("edges", face.edges.iter().map(|e| e.0).collect::<Vec<_>>());
            f.set_property("num_buildings", face.num_buildings);
            features.push(f);
        }
        serde_json::to_string(&GeoJson::from(features)).map_err(err_to_js)
    }

    #[wasm_bindgen(js_name = getBuildings)]
    pub fn get_buildings(&self) -> Result<String, JsValue> {
        let mut features = Vec::new();
        for pt in &self.building_centroids {
            features.push(self.graph.mercator.to_wgs84_gj(pt));
        }
        serde_json::to_string(&GeoJson::from(features)).map_err(err_to_js)
    }
}

fn err_to_js<E: std::fmt::Display>(err: E) -> JsValue {
    JsValue::from_str(&err.to_string())
}

fn keep_edge(tags: &Tags) -> bool {
    if !tags.has("highway") || tags.is("highway", "proposed") || tags.is("area", "yes") {
        return false;
    }
    /*if tags.is_any("highway", vec!["footway", "cycleway"]) {
        return false;
    }*/
    true
}
