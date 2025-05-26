use std::collections::{BTreeMap, BTreeSet};

use geo::{
    BoundingRect, Centroid, Contains, Coord, Distance, Euclidean, InterpolatableLine, LineString,
    Point, Polygon, Rect,
};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::float::slice::FloatSlice;
use rstar::{primitives::GeomWithData, RTree, AABB};
use utils::osm2graph::{EdgeID, Graph, Intersection, IntersectionID};

use crate::{slice_nearest_boundary::SliceNearEndpoints, Command, RoadBundler};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct FaceID(pub usize);

pub struct Face {
    pub polygon: Polygon,
    pub boundary_edges: Vec<EdgeID>,
    pub boundary_intersections: Vec<IntersectionID>,
    pub connecting_edges: Vec<EdgeID>,
    pub num_buildings: usize,
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

        let id = FaceID(faces.len());
        faces.insert(
            id,
            Face {
                polygon,
                boundary_edges,
                boundary_intersections,
                connecting_edges,
                num_buildings,
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
    pub fn apply_cmd(&mut self, cmd: Command) {
        match cmd {
            Command::CollapseToCentroid(face) => self.collapse_to_centroid(face),
        }
        self.faces = make_faces(&self.graph, &self.building_centroids);
    }

    fn collapse_to_centroid(&mut self, id: FaceID) {
        let face = &self.faces[&id];

        for e in &face.boundary_edges {
            remove_edge(&mut self.graph, *e);
        }

        // Create a new intersection at the centroid
        let new_intersection = next_intersection_id(&self.graph);
        self.graph.intersections.insert(
            new_intersection,
            Intersection {
                id: new_intersection,
                edges: Vec::new(),
                // TODO Need a diff graph struct, to allow for mixed synthetic and OSM
                osm_node: osm_reader::NodeID(0),
                point: face.polygon.centroid().expect("no face centroid"),
            },
        );

        for i in &face.boundary_intersections {
            // Remove this intersection, asserting there's one surviving edge and connecting it
            // instead to the new intersection. That edge is in connecting_edges, but we don't need
            // that list.
            replace_intersection(&mut self.graph, *i, new_intersection);
        }

        // TODO Do we need to compact_ids again?
    }
}

fn remove_edge(graph: &mut Graph, e: EdgeID) {
    let edge = graph
        .edges
        .remove(&e)
        .expect("can't remove edge that doesn't exist");
    for i in [edge.src, edge.dst] {
        let intersection = graph.intersections.get_mut(&i).unwrap();
        intersection.edges.retain(|x| *x != e);
        // If edge.src == edge.dst, this is idempotent
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

fn next_intersection_id(graph: &Graph) -> IntersectionID {
    IntersectionID(graph.intersections.keys().max().unwrap().0 + 1)
}
