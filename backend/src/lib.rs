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
use utils::Tags;
use wasm_bindgen::prelude::*;

use crate::debugger::Debugger;
use crate::faces::{make_faces, Face, FaceID, FaceKind};
use crate::graph::{EdgeID, Graph, Intersection, IntersectionID, IntersectionProvenance};

mod average_lines;
mod debugger;
mod dog_leg;
mod dual_carriageway;
mod faces;
mod graph;
mod join_lines;
mod scrape_buildings;
mod sidepath;
mod slice_nearest_boundary;
mod split_line;

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
        let mut osm_graph = utils::osm2graph::Graph::new(input_bytes, keep_edge, &mut buildings)
            .map_err(err_to_js)?;
        osm_graph.compact_ids();
        let graph = Graph::new(osm_graph);

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
            f.set_property(
                "provenance",
                serde_json::to_value(&edge.provenance).map_err(err_to_js)?,
            );
            f.set_property("is_road", !edge.is_sidewalk_or_cycleway());
            features.push(f);
        }
        serde_json::to_string(&GeoJson::from(features)).map_err(err_to_js)
    }

    #[wasm_bindgen(js_name = getIntersections)]
    pub fn get_intersections(&self) -> Result<String, JsValue> {
        let mut features = Vec::new();
        for (id, i) in &self.graph.intersections {
            let mut f = self.graph.mercator.to_wgs84_gj(&i.point);
            f.set_property("intersection_id", id.0);
            f.set_property(
                "provenance",
                serde_json::to_value(&i.provenance).map_err(err_to_js)?,
            );
            features.push(f);
        }
        serde_json::to_string(&GeoJson::from(features)).map_err(err_to_js)
    }

    #[wasm_bindgen(js_name = getOriginalOsmGraph)]
    pub fn get_original_osm_graph(&self) -> Result<String, JsValue> {
        let mut features = Vec::new();
        for (_, edge) in &self.original_graph.edges {
            features.push(self.graph.mercator.to_wgs84_gj(&edge.linestring));
        }
        for (_, i) in &self.original_graph.intersections {
            features.push(self.graph.mercator.to_wgs84_gj(&i.point));
        }
        serde_json::to_string(&GeoJson::from(features)).map_err(err_to_js)
    }

    #[wasm_bindgen(js_name = getFaces)]
    pub fn get_faces(&self) -> Result<String, JsValue> {
        let mut features = Vec::new();
        for (id, face) in &self.faces {
            features.push(face.to_gj(&self.graph, *id));
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

    #[wasm_bindgen(js_name = collapseDualCarriageway)]
    pub fn collapse_dual_carriageway_wasm(&mut self, id: usize) {
        let cmd = Command::CollapseDualCarriageway(FaceID(id));
        self.commands.push(cmd);
        self.apply_cmd(cmd);
    }

    /// Returns the number of new commands applied
    #[wasm_bindgen(js_name = fixAllDualCarriageways)]
    pub fn fix_all_dual_carriageways(&mut self) -> usize {
        let mut cmds_applied = 0;

        loop {
            if let Some((id, _)) = self.faces.iter().find(|(_, face)| {
                crate::dual_carriageway::DualCarriageway::maybe_new(&self.graph, face).is_ok()
            }) {
                let cmd = Command::CollapseDualCarriageway(*id);
                self.commands.push(cmd);
                self.apply_cmd(cmd);
                cmds_applied += 1;
            } else {
                break;
            }
        }

        cmds_applied
    }

    #[wasm_bindgen(js_name = mergeSidepath)]
    pub fn merge_sidepath_wasm(&mut self, id: usize) {
        let cmd = Command::MergeSidepath(FaceID(id));
        self.commands.push(cmd);
        self.apply_cmd(cmd);
    }

    /// Returns the number of new commands applied
    #[wasm_bindgen(js_name = fixAllSidepaths)]
    pub fn fix_all_sidepaths(&mut self) -> usize {
        let mut cmds_applied = 0;

        loop {
            if let Some((id, _)) = self
                .faces
                .iter()
                .find(|(_, face)| face.kind == FaceKind::SidepathArtifact)
            {
                let cmd = Command::MergeSidepath(*id);
                self.commands.push(cmd);
                self.apply_cmd(cmd);
                cmds_applied += 1;
            } else {
                break;
            }
        }

        cmds_applied
    }

    #[wasm_bindgen(js_name = collapseEdge)]
    pub fn collapse_edge_wasm(&mut self, id: usize) {
        let cmd = Command::CollapseEdge(EdgeID(id));
        self.commands.push(cmd);
        self.apply_cmd(cmd);
    }

    /// Returns the number of new commands applied
    #[wasm_bindgen(js_name = fixAllDogLegs)]
    pub fn fix_all_dog_legs(&mut self) -> usize {
        let mut cmds_applied = 0;

        loop {
            if let Some(id) = self.graph.edges.keys().find(|e| self.is_dog_leg(**e)) {
                let cmd = Command::CollapseEdge(*id);
                self.commands.push(cmd);
                self.apply_cmd(cmd);
                cmds_applied += 1;
            } else {
                break;
            }
        }

        cmds_applied
    }
}

impl RoadBundler {
    pub fn apply_cmd(&mut self, cmd: Command) {
        match cmd {
            Command::CollapseToCentroid(face) => self.collapse_to_centroid(face),
            Command::CollapseDualCarriageway(face) => self.collapse_dual_carriageway(face),
            Command::MergeSidepath(face) => self.merge_sidepath(face),
            Command::CollapseEdge(edge) => self.collapse_edge(edge),
        }
        self.faces = make_faces(&self.graph, &self.building_centroids);
    }
}

// IDs are only meaningful when applied in the correct order
#[derive(Clone, Copy)]
pub enum Command {
    CollapseToCentroid(FaceID),
    CollapseDualCarriageway(FaceID),
    MergeSidepath(FaceID),
    CollapseEdge(EdgeID),
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
