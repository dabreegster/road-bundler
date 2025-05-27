#[allow(unused_imports)]
#[macro_use]
extern crate anyhow;
#[allow(unused_imports)]
#[macro_use]
extern crate log;

use std::collections::BTreeMap;
use std::sync::Once;

use anyhow::Result;
use geo::{Centroid, Point};
use geojson::GeoJson;
use utils::{osm2graph::Graph, Tags};
use wasm_bindgen::prelude::*;

use crate::faces::{make_faces, Face, FaceID};

mod dual_carriageway;
mod faces;
mod join_lines;
mod scrape_buildings;
mod slice_nearest_boundary;

static START: Once = Once::new();

#[wasm_bindgen]
pub struct RoadBundler {
    original_graph: Graph,
    building_centroids: Vec<Point>,
    commands: Vec<Command>,

    // Derived
    graph: Graph,
    faces: BTreeMap<FaceID, Face>,
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
            original_graph: graph.clone(),
            building_centroids,
            commands: Vec::new(),

            graph,
            faces,
        })
    }

    #[wasm_bindgen(js_name = getEdges)]
    pub fn get_edges(&self) -> Result<String, JsValue> {
        let mut features = Vec::new();
        for (id, edge) in &self.graph.edges {
            let mut f = self.graph.mercator.to_wgs84_gj(&edge.linestring);
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
        for (id, face) in &self.faces {
            let mut f = self.graph.mercator.to_wgs84_gj(&face.polygon);
            f.set_property("face_id", id.0);
            f.set_property(
                "boundary_edges",
                face.boundary_edges.iter().map(|e| e.0).collect::<Vec<_>>(),
            );
            f.set_property(
                "boundary_intersections",
                face.boundary_intersections
                    .iter()
                    .map(|i| {
                        serde_json::to_value(
                            &self
                                .graph
                                .mercator
                                .to_wgs84_gj(&Point::from(self.graph.intersections[i].point)),
                        )
                        .unwrap()
                    })
                    .collect::<Vec<_>>(),
            );
            f.set_property(
                "connecting_edges",
                face.connecting_edges
                    .iter()
                    .map(|e| e.0)
                    .collect::<Vec<_>>(),
            );
            f.set_property("num_buildings", face.num_buildings);
            if let Some(dc) = dual_carriageway::DualCarriageway::maybe_new(&self.graph, face) {
                f.set_property("dual_carriageway", serde_json::to_value(&dc).unwrap());
            }
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

    #[wasm_bindgen(js_name = undo)]
    pub fn undo(&mut self) {
        self.commands.pop();
        self.graph = self.original_graph.clone();
        self.faces = make_faces(&self.graph, &self.building_centroids);

        for cmd in self.commands.clone() {
            self.apply_cmd(cmd);
        }
    }

    #[wasm_bindgen(js_name = collapseToCentroid)]
    pub fn collapse_to_centroid_wasm(&mut self, id: usize) {
        let cmd = Command::CollapseToCentroid(FaceID(id));
        self.commands.push(cmd);
        self.apply_cmd(cmd);
    }
}

// IDs are only meaningful when applied in the correct order
#[derive(Clone, Copy)]
pub enum Command {
    CollapseToCentroid(FaceID),
}

fn err_to_js<E: std::fmt::Display>(err: E) -> JsValue {
    JsValue::from_str(&err.to_string())
}

fn keep_edge(tags: &Tags) -> bool {
    if !tags.has("highway") || tags.is("highway", "proposed") || tags.is("area", "yes") {
        return false;
    }
    if tags.is_any("highway", vec!["footway", "cycleway"]) {
        return false;
    }
    true
}
