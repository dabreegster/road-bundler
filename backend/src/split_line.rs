use geo::{Closest, ClosestPoint, LineLocatePoint, LineString, Point};
use utils::{osm2graph::Graph, LineSplit};

use crate::Face;

pub struct Splits {
    pub split_pts: Vec<Point>,
    pub lines: Vec<LineString>,
}

pub fn split_center(graph: &Graph, center_line: &LineString, face: &Face) -> Splits {
    // Where does this new center need to be split, based on connecting edges? Side roads will be
    // properly in the middle, and connections (maybe resolved by first merging the faces) will be
    // at 0 or 1.
    let mut split_fractions = Vec::new();
    let mut split_pts = Vec::new();
    for e in &face.connecting_edges {
        let edge = &graph.edges[e];
        let i = if face.boundary_intersections.contains(&edge.src) {
            edge.src
        } else {
            edge.dst
        };

        let split_pt = match center_line.closest_point(&graph.intersections[&i].point) {
            Closest::Intersection(pt) => pt,
            Closest::SinglePoint(pt) => pt,
            Closest::Indeterminate => {
                // TODO Possible?
                panic!("closest_point for split_center is Indeterminate");
            }
        };
        split_pts.push(split_pt);

        split_fractions.extend(center_line.line_locate_point(&split_pt));
    }
    split_fractions.sort_by_key(|x| (*x * 10e9) as usize);
    split_fractions.dedup();
    let lines: Vec<LineString> = center_line
        .line_split_many(&split_fractions)
        .unwrap()
        .into_iter()
        .flatten()
        .collect();
    info!(
        "split_fractions {split_fractions:?} yields {} lines",
        lines.len()
    );

    Splits { split_pts, lines }
}
