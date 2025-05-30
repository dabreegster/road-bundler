use std::collections::{BTreeMap, BTreeSet};

use geo::{
    BoundingRect, Centroid, Contains, Coord, Distance, Euclidean, InterpolatableLine, LineString,
    Point, Polygon, Rect,
};
use geojson::Feature;
use i_overlay::core::fill_rule::FillRule;
use i_overlay::float::slice::FloatSlice;
use rstar::{primitives::GeomWithData, RTree, AABB};

use crate::{
    slice_nearest_boundary::SliceNearEndpoints, Debugger, EdgeID, Graph, Intersection,
    IntersectionID, IntersectionProvenance, RoadBundler,
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

pub fn make_faces(graph: &Graph, building_centroids: &Vec<Point>) -> BTreeMap<FaceID, Face> {
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
        let boundary_edges = closest_edge
            .locate_in_envelope_intersecting(&aabb(&polygon))
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
            .iter()
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
    midpoint_distance(ls, &slice1) <= 1.0 || midpoint_distance(ls, &slice2) <= 1.0
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
            // Remove this intersection, asserting there's one surviving edge and connecting it
            // instead to the new intersection. That edge is in connecting_edges, but we don't need
            // that list.
            replace_intersection(&mut self.graph, *i, new_intersection);
        }
    }
}

fn replace_intersection(
    graph: &mut Graph,
    remove_i: IntersectionID,
    new_intersection: IntersectionID,
) {
    let intersection = graph
        .intersections
        .remove(&remove_i)
        .expect("can't remove intersection that doesn't exist");
    // There could be 0 of these, fine
    // Usually there's 1
    // But there could also be multiple -- a roundabout with two roads jutting off from the same
    // node. Fine.
    for surviving_edge in intersection.edges {
        let edge = graph.edges.get_mut(&surviving_edge).unwrap();
        let mut updated = false;
        if edge.src == remove_i {
            edge.src = new_intersection;
            // TODO In provenance, should we mark modified cases?
            edge.linestring
                .0
                .insert(0, graph.intersections[&new_intersection].point.into());
            updated = true;
        }
        if edge.dst == remove_i {
            edge.dst = new_intersection;
            edge.linestring
                .0
                .push(graph.intersections[&new_intersection].point.into());
            updated = true;
        }

        if updated {
            graph
                .intersections
                .get_mut(&new_intersection)
                .unwrap()
                .edges
                .push(surviving_edge);
        } else {
            panic!("replace_intersection saw inconsistent state about an edge connected to an intersection");
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
        f
    }
}
