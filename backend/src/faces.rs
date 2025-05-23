use geo::{Coord, LineString, Polygon, Relate};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::float::slice::FloatSlice;
use utils::osm2graph::{EdgeID, Graph};

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
                // TODO is_touches?
                if edge.linestring.relate(&polygon).is_intersects() {
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
