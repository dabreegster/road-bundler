use std::collections::{BTreeMap, BTreeSet};

use geo::{
    BoundingRect, Centroid, Contains, Coord, Distance, Euclidean, InterpolatableLine, LineString,
    Point, Polygon, Rect,
};
use geojson::Feature;
use i_overlay::core::fill_rule::FillRule;
use i_overlay::float::slice::FloatSlice;
use rstar::{primitives::GeomWithData, RTree, AABB};

use crate::geo_helpers::SliceNearEndpoints;
use crate::{
    Debugger, EdgeID, Graph, Intersection, IntersectionID, IntersectionProvenance, RoadBundler,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct FaceID(pub usize);

pub struct Face {
    pub polygon: Polygon,
    pub kind: FaceKind,
    pub boundary_edges: Vec<EdgeID>,
    pub boundary_intersections: Vec<IntersectionID>,
    pub connecting_edges: Vec<EdgeID>,
}

#[derive(Debug, PartialEq)]
pub enum FaceKind {
    /// Should be not be simplified. There are buildings or real land uses inside. This also
    /// includes parking aisles.
    UrbanBlock,
    /// This is a candidate for being simplified
    RoadArtifact,
    /// The space between a road and a sidewalk or cyclepath
    SidepathArtifact,
}

pub fn make_faces(graph: &Graph, building_centroids: &RTree<Point>) -> BTreeMap<FaceID, Face> {
    info!("Splitting {} edges into faces", graph.edges.len());
    let polygons = split_polygon(
        &graph.boundary_polygon,
        graph.edges.values().map(|edge| &edge.linestring),
    );

    info!("Building rtree for {} edges", graph.edges.len());
    let closest_edge = RTree::bulk_load(
        graph
            .edges
            .values()
            .map(|e| GeomWithData::new(e.linestring.clone(), e.id))
            .collect(),
    );

    info!("Matching {} faces with edges", polygons.len());
    let mut faces = BTreeMap::new();
    for polygon in polygons {
        let bbox = aabb(&polygon);

        let boundary_edges = closest_edge
            .locate_in_envelope_intersecting(&bbox)
            .filter_map(|obj| {
                if linestring_along_polygon(obj.geom(), &polygon) {
                    Some(obj.data)
                } else {
                    None
                }
            })
            .collect();
        let (boundary_intersections, connecting_edges) = find_connections(graph, &boundary_edges);
        let num_buildings = building_centroids
            .locate_in_envelope_intersecting(&bbox)
            .filter(|pt| polygon.contains(*pt))
            .count();
        let has_parking_aisle = boundary_edges
            .iter()
            .any(|e| graph.edges[e].is_parking_aisle());
        let mut num_roads = 0;
        let mut num_non_roads = 0;
        for e in &boundary_edges {
            if graph.edges[e].is_sidewalk_or_cycleway() {
                num_non_roads += 1;
            } else {
                num_roads += 1;
            }
        }
        let kind = if num_buildings > 0 {
            FaceKind::UrbanBlock
        } else if num_roads > 0 && num_non_roads > 0 {
            FaceKind::SidepathArtifact
        } else if has_parking_aisle {
            // Order matters -- sidepaths next to a parking aisle count as SidepathArtifact
            FaceKind::UrbanBlock
        } else {
            FaceKind::RoadArtifact
        };

        let id = FaceID(faces.len());
        faces.insert(
            id,
            Face {
                polygon,
                kind,
                boundary_edges,
                boundary_intersections,
                connecting_edges,
            },
        );
    }
    info!("Done");
    faces
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
            let mut linestrings: Vec<LineString> =
                rings.into_iter().map(to_geo_linestring).collect();
            if linestrings.is_empty() {
                panic!("a split shape is empty");
            }
            let exterior = linestrings.remove(0);
            Polygon::new(exterior, linestrings)
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

fn linestring_along_polygon(ls: &LineString, polygon: &Polygon) -> bool {
    // If there are holes, treat each of them as its own polygon
    // TODO Not working in St Mary's Gardens
    for hole in polygon.interiors() {
        if linestring_along_polygon(ls, &Polygon::new(hole.clone(), Vec::new())) {
            return true;
        }
    }

    let (slice1, slice2) = polygon.slice_near_endpoints(ls);

    // TODO Pick the more appropriate slice, using length?
    let threshold = 1.5;
    midpoint_distance(ls, &slice1) <= threshold || midpoint_distance(ls, &slice2) <= threshold
}

// Distance in meters between the middle of each linestring. Because ls1 and ls2 might point
// opposite directions, using the start/end point is unnecessarily trickier.
fn midpoint_distance(ls1: &LineString, ls2: &LineString) -> f64 {
    let pt1 = ls1.point_at_ratio_from_start(&Euclidean, 0.5).unwrap();
    let pt2 = ls2.point_at_ratio_from_start(&Euclidean, 0.5).unwrap();
    Euclidean.distance(pt1, pt2)
}

fn aabb<G: BoundingRect<f64, Output = Option<Rect<f64>>>>(geom: &G) -> AABB<Point> {
    let bbox: Rect = geom.bounding_rect().unwrap().into();
    AABB::from_corners(
        Point::new(bbox.min().x, bbox.min().y),
        Point::new(bbox.max().x, bbox.max().y),
    )
}

fn find_connections(
    graph: &Graph,
    boundary_edges: &Vec<EdgeID>,
) -> (Vec<IntersectionID>, Vec<EdgeID>) {
    let mut boundary_intersections = BTreeSet::new();
    for edge in boundary_edges {
        let edge = &graph.edges[edge];
        for i in [edge.src, edge.dst] {
            boundary_intersections.insert(i);
        }
    }

    let mut connecting_edges = BTreeSet::new();
    for i in &boundary_intersections {
        connecting_edges.extend(graph.intersections[i].edges.clone());
    }
    for e in boundary_edges {
        connecting_edges.remove(e);
    }

    (
        boundary_intersections.into_iter().collect(),
        connecting_edges.into_iter().collect(),
    )
}

impl RoadBundler {
    pub fn collapse_to_centroid(&mut self, id: FaceID) {
        let face = &self.faces[&id];

        // TODO Preserve associated_original_edges, probably on the new intersection
        for e in &face.boundary_edges {
            self.graph.remove_edge(*e);
        }

        // Create a new intersection at the centroid
        let new_intersection = self.graph.new_intersection_id();
        self.graph.intersections.insert(
            new_intersection,
            Intersection {
                id: new_intersection,
                edges: Vec::new(),
                point: face.polygon.centroid().expect("no face centroid"),
                provenance: IntersectionProvenance::Synthetic,
            },
        );

        for i in &face.boundary_intersections {
            // Remove this intersection, reconnecting the surviving edges instead to the new
            // intersection. (Those edges are in connecting_edges, but we don't need that list.)
            //
            // Usually there's just 1 surviving edge, but there could be 0, or a roundabout with
            // two roads jutting off from the same node.
            let extend_geometry = true;
            self.graph
                .replace_intersection(*i, new_intersection, extend_geometry);
        }
    }
}

impl Face {
    pub fn to_gj(&self, graph: &Graph, id: FaceID) -> Feature {
        let mut debug_hover = Debugger::new(graph.mercator.clone());
        for e in &self.boundary_edges {
            debug_hover.line(&graph.edges[e].linestring, "boundary edge", "red", 5, 1.0);
        }
        for e in &self.connecting_edges {
            debug_hover.line(
                &graph.edges[e].linestring,
                "connecting edge",
                "yellow",
                5,
                1.0,
            );
        }

        let mut f = graph.mercator.to_wgs84_gj(&self.polygon);
        f.set_property("face_id", id.0);
        f.set_property("debug_hover", debug_hover.build());
        f.set_property("kind", format!("{:?}", self.kind));
        match crate::dual_carriageway::DualCarriageway::maybe_new(graph, self) {
            Ok(dc) => f.set_property("dual_carriageway", serde_json::to_value(&dc).unwrap()),
            Err(err) => f.set_property("dual_carriageway", err.to_string()),
        }
        match crate::sidepath::detect_sidepath(graph, self) {
            Ok(gj) => f.set_property("sidepath", serde_json::to_value(&gj).unwrap()),
            Err(err) => f.set_property("sidepath", err.to_string()),
        }
        f
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::wkt;

    #[test]
    fn test_linestring_along_polygon() {
        let polygon = wkt!(POLYGON((190.27018273197035 40.652921341231966,190.64442074619154 48.99255242185941,190.76417696796278 53.86289682226529,195.98105752788405 65.92756309347502,200.80124533496718 54.8858915789639,200.82369983516554 49.55964722471585,200.8087302478395 41.15329923467984,200.50185478053908 32.35776843862882,200.09767759166579 23.962539814284945,195.78645360790114 17.746734760574007,190.76417696796278 25.408075950912142,190.49472558818678 33.10277547674527,190.27018273197035 40.652921341231966)));
        let ls = wkt!(LINESTRING(190.27018264451632 40.65292133361041,190.64442083873556 48.99255235061214,190.76417706118355 53.862896864869825,195.98105748674922 65.92756307056125));
        assert!(linestring_along_polygon(&ls, &polygon));
    }
}
