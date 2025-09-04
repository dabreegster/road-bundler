use std::collections::{BTreeMap, BTreeSet};

use geo::buffer::{BufferStyle, LineJoin};
use geo::{
    BooleanOps, BoundingRect, Buffer, Centroid, Contains, Distance, Euclidean, InterpolatableLine,
    LineString, MultiLineString, Point, Polygon, Rect,
};
use geojson::Feature;
use rstar::{primitives::GeomWithData, RTree, AABB};
use utils::split_polygon;

use crate::geo_helpers::SliceNearEndpoints;
use crate::{
    Areas, Debugger, EdgeID, EdgeKind, Graph, Intersection, IntersectionID, IntersectionProvenance,
    RoadBundler,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct FaceID(pub usize);

pub struct Face {
    pub polygon: Polygon,
    pub kind: FaceKind,
    pub boundary_edges: Vec<EdgeID>,
    pub boundary_intersections: Vec<IntersectionID>,
    pub connecting_edges: Vec<EdgeID>,
    // TODO Some connecting_edges also belong in here, but not detecting yet
    pub internal_edges: Vec<EdgeID>,
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
    /// Water or a park -- and no roads inside?
    OtherArea,
}

pub fn make_faces(graph: &Graph, areas: &Areas) -> BTreeMap<FaceID, Face> {
    info!("Splitting {} edges into faces", graph.edges.len());
    let polygons = split_polygon(
        &graph.boundary_polygon,
        graph.edges.values().map(|edge| &edge.linestring).chain(
            areas
                .other_polygons
                .iter()
                .map(|polygon| polygon.exterior()),
        ),
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

        let mut boundary_edges = Vec::new();
        let mut internal_edges = Vec::new();
        for obj in closest_edge.locate_in_envelope_intersecting(&bbox) {
            if linestring_along_polygon(obj.geom(), &polygon) {
                boundary_edges.push(obj.data);
            } else if polygon.contains(obj.geom()) {
                internal_edges.push(obj.data);
            }
        }

        let (boundary_intersections, connecting_edges) = find_connections(graph, &boundary_edges);
        let num_buildings = areas
            .building_centroids
            .locate_in_envelope_intersecting(&bbox)
            .filter(|pt| polygon.contains(*pt))
            .count();
        let num_other_areas = areas
            .other_centroids
            .locate_in_envelope_intersecting(&bbox)
            .filter(|pt| polygon.contains(*pt))
            .count();
        let has_parking_aisle = boundary_edges
            .iter()
            .any(|e| graph.edges[e].kind.is_parking_aisle(graph));
        let mut num_roads = 0;
        let mut num_non_roads = 0;
        for e in &boundary_edges {
            match graph.edges[e].kind {
                EdgeKind::Motorized { .. } => {
                    num_roads += 1;
                }
                EdgeKind::Nonmotorized(_) => {
                    num_non_roads += 1;
                }
            }
        }
        let kind = if num_buildings > 0 {
            FaceKind::UrbanBlock
        } else if num_other_areas > 0 {
            FaceKind::OtherArea
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
                internal_edges,
            },
        );
    }
    info!("Done");
    faces
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

        // TODO How/where do we preserve associations? Edges are becoming a node
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
        for e in &self.internal_edges {
            debug_hover.line(&graph.edges[e].linestring, "internal edge", "blue", 5, 1.0);
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
        f.set_property(
            "generated_sidewalks",
            self.generated_sidewalks(graph).build(),
        );
        f
    }

    fn generated_sidewalks(&self, graph: &Graph) -> Debugger {
        let mut debug = Debugger::new(graph.mercator.clone());
        let width = 3.0;

        let mut subtract = Vec::new();
        // internal_edges isn't complete, so include all connecting_edges too. The ones that're
        // outside this face won't matter; we're subtracting anyway.
        for e in self
            .boundary_edges
            .iter()
            .chain(self.connecting_edges.iter())
            .chain(self.internal_edges.iter())
        {
            subtract.push(graph.edges[e].linestring.clone());
        }
        // Buffer these all in one batch; it's much cleaner
        let subtract_polygons = MultiLineString(subtract)
            .buffer_with_style(BufferStyle::new(width).line_join(LineJoin::Round(width)));

        let combo = self.polygon.difference(&subtract_polygons);
        for polygon in combo {
            //debug.polygon(&polygon, "generated sidewalk", "green", 0.8);
            debug.line(polygon.exterior(), "generated sidewalk", "green", 2, 1.0);
        }

        /*for polygon in subtract_polygons {
            debug.polygon(&polygon, "subtract", "red", 0.8);
        }*/

        debug
    }

    // This first negative-buffers the face, then subtracts internal stuff. No real advantage?
    #[allow(unused)]
    fn generated_sidewalks_alt(&self, graph: &Graph) -> Debugger {
        let mut debug = Debugger::new(graph.mercator.clone());
        let width = 3.0;

        let mut subtract = Vec::new();
        // internal_edges isn't complete, so include all connecting_edges too. The ones that're
        // outside this face won't matter; we're subtracting anyway.
        for e in self
            .connecting_edges
            .iter()
            .chain(self.internal_edges.iter())
        {
            subtract.push(graph.edges[e].linestring.clone());
        }
        // Buffer these all in one batch; it's much cleaner
        let subtract_polygons = MultiLineString(subtract).buffer(width);

        let shrunken_face = self.polygon.buffer(-width);

        /*for polygon in shrunken_face {
            debug.polygon(&polygon, "generated sidewalk", "green", 1.0);
        }
        for polygon in subtract_polygons {
            debug.polygon(&polygon, "subtract", "red", 0.8);
        }*/

        let combo = shrunken_face.difference(&subtract_polygons);
        for polygon in combo {
            //debug.polygon(&polygon, "generated sidewalk", "green", 0.8);
            debug.line(polygon.exterior(), "generated sidewalk", "green", 2, 1.0);
        }

        debug
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
