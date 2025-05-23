#[allow(unused_imports)]
#[macro_use]
extern crate anyhow;
#[allow(unused_imports)]
#[macro_use]
extern crate log;

use std::sync::Once;

use anyhow::Result;
use geo::{Coord, LineString, Polygon};
use geojson::GeoJson;
use i_overlay::core::fill_rule::FillRule;
use i_overlay::float::slice::FloatSlice;
use utils::{osm2graph::Graph, Tags};
use wasm_bindgen::prelude::*;

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
            features.push(self.graph.mercator.to_wgs84_gj(&face.polygon));
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

struct Face {
    polygon: Polygon,
}

fn make_faces(graph: &Graph) -> Vec<Face> {
    let polygons = split_polygon(
        &graph.boundary_polygon,
        graph.edges.values().map(|edge| &edge.linestring),
    );
    polygons
        .into_iter()
        .map(|polygon| Face { polygon })
        .collect()
}

// TODO Revisit some of this; conversions are now in geo
fn split_polygon<'a>(
    polygon: &Polygon,
    lines: impl Iterator<Item = &'a LineString>,
) -> Vec<Polygon> {
    let mut shape = to_i_overlay_contour(polygon.exterior());

    // geo Polygon's are explicitly closed LineStrings, but i_overlay Polygon's are not.
    shape.pop();

    let splitters: Vec<_> = lines.map(to_i_overlay_contour).collect();
    let shapes = shape.slice_by(&splitters, FillRule::NonZero);

    shapes
        .into_iter()
        .map(|rings| {
            let exterior = rings.into_iter().next().expect("shapes must be non-empty");
            let exterior_line_string = to_geo_linestring(exterior);
            // We ignore any interiors
            Polygon::new(exterior_line_string, vec![])
        })
        .collect()
}

fn to_geo_linestring(pts: Vec<[f64; 2]>) -> LineString {
    LineString(
        pts.into_iter()
            .map(|pt| Coord { x: pt[0], y: pt[1] })
            .collect(),
    )
}

fn to_i_overlay_contour(line_string: &LineString) -> Vec<[f64; 2]> {
    line_string.coords().map(|c| [c.x, c.y]).collect()
}
