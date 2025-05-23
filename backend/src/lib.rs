#[allow(unused_imports)]
#[macro_use]
extern crate anyhow;
#[allow(unused_imports)]
#[macro_use]
extern crate log;

use std::sync::Once;

use anyhow::Result;
use geojson::GeoJson;
use utils::{osm2graph::Graph, Tags};
use wasm_bindgen::prelude::*;

use crate::faces::{make_faces, Face};

mod faces;

static START: Once = Once::new();

#[wasm_bindgen]
pub struct RoadBundler {
    graph: Graph,
    faces: Vec<Face>,
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

        let graph =
            utils::osm2graph::Graph::new(input_bytes, keep_edge, &mut utils::osm2graph::NullReader)
                .map_err(err_to_js)?;
        let faces = make_faces(&graph);
        Ok(Self { graph, faces })
    }

    #[wasm_bindgen(js_name = getWays)]
    pub fn get_ways(&self) -> Result<String, JsValue> {
        let mut features = Vec::new();
        for (id, edge) in &self.graph.edges {
            let mut f = self.graph.mercator.to_wgs84_gj(&edge.linestring);
            f.id = Some(geojson::feature::Id::Number(id.0.into()));
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
            features.push(f);
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
    true
}
