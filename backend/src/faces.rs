use geo::{
    BoundingRect, Contains, Coord, Distance, Euclidean, InterpolatableLine, LineString, Point,
    Polygon, Rect,
};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::float::slice::FloatSlice;
use rstar::{primitives::GeomWithData, RTree, AABB};
use utils::osm2graph::{EdgeID, Graph};

use crate::slice_nearest_boundary::SliceNearEndpoints;

pub struct Face {
    pub polygon: Polygon,
    pub edges: Vec<EdgeID>,
    pub num_buildings: usize,
}

pub fn make_faces(graph: &Graph, building_centroids: &Vec<Point>) -> Vec<Face> {
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
    let mut faces = Vec::new();
    for polygon in polygons {
        let edges = closest_edge
            .locate_in_envelope_intersecting(&aabb(&polygon))
            .filter_map(|obj| {
                if linestring_along_polygon(obj.geom(), &polygon) {
                    Some(obj.data)
                } else {
                    None
                }
            })
            .collect();
        let num_buildings = building_centroids
            .iter()
            .filter(|pt| polygon.contains(*pt))
            .count();

        faces.push(Face {
            polygon,
            edges,
            num_buildings,
        });
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

fn linestring_along_polygon(ls: &LineString, polygon: &Polygon) -> bool {
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
