use geo::{LineString, Point};
use geojson::{Feature, GeoJson};
use utils::Mercator;

/// Generates a FeatureCollection that can be rendered as one simple layer in maplibre
pub struct Debugger {
    mercator: Mercator,
    features: Vec<Feature>,
}

impl Debugger {
    pub fn new(mercator: Mercator) -> Self {
        Self {
            mercator,
            features: Vec::new(),
        }
    }

    pub fn line(&mut self, ls: &LineString, label: &str, color: &str, width: usize, opacity: f64) {
        let mut f = self.mercator.to_wgs84_gj(ls);
        f.set_property("label", label);
        f.set_property("color", color);
        f.set_property("width", width);
        f.set_property("opacity", opacity);
        self.features.push(f);
    }

    pub fn circle(&mut self, pt: Point, label: &str, color: &str, radius: usize) {
        let mut f = self.mercator.to_wgs84_gj(&pt);
        f.set_property("label", label);
        f.set_property("color", color);
        f.set_property("radius", radius);
        self.features.push(f);
    }

    pub fn build(self) -> GeoJson {
        GeoJson::from(self.features)
    }
}
