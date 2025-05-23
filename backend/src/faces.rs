use geo::{Coord, Distance, Euclidean, InterpolatableLine, LineLocatePoint, LineString, Polygon};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::float::slice::FloatSlice;
use utils::{
    osm2graph::{EdgeID, Graph},
    LineSplit,
};

pub struct Face {
    pub polygon: Polygon,
    pub edges: Vec<EdgeID>,
}

pub fn make_faces(graph: &Graph) -> Vec<Face> {
    info!("Splitting {} edges into faces", graph.edges.len());
    let polygons = split_polygon(
        &graph.boundary_polygon,
        graph.edges.values().map(|edge| &edge.linestring),
    );

    info!("Matching {} faces with edges", polygons.len());
    let mut faces = Vec::new();
    for polygon in polygons {
        // TODO Speed up
        let edges = graph
            .edges
            .values()
            .filter_map(|edge| {
                if linestring_along_polygon(&edge.linestring, &polygon) {
                    Some(edge.id)
                } else {
                    None
                }
            })
            .collect();
        faces.push(Face { polygon, edges });
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
    let Some(slice) = slice_line_to_match(polygon.exterior(), ls) else {
        return false;
    };
    midpoint_distance(ls, &slice) <= 1.0
}

// Slice `source` to correspond to `target`, by finding the closest point along `source` matching
// `target`'s start and end point.
fn slice_line_to_match(source: &LineString, target: &LineString) -> Option<LineString> {
    let start = source.line_locate_point(&target.points().next().unwrap())?;
    let end = source.line_locate_point(&target.points().last().unwrap())?;
    // Note this uses a copy of an API that hasn't been merged into georust yet. It seems to work
    // fine in practice.
    source.line_split_twice(start, end)?.into_second()
}

// Distance in meters between the middle of each linestring. Because ls1 and ls2 might point
// opposite directions, using the start/end point is unnecessarily trickier.
fn midpoint_distance(ls1: &LineString, ls2: &LineString) -> f64 {
    let pt1 = ls1.point_at_ratio_from_start(&Euclidean, 0.5).unwrap();
    let pt2 = ls2.point_at_ratio_from_start(&Euclidean, 0.5).unwrap();
    Euclidean.distance(pt1, pt2)
}
