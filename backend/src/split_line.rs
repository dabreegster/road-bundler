use geo::{Closest, ClosestPoint, LineLocatePoint, LineString, Point};
use utils::LineSplit;

use crate::{Face, Graph};

pub struct Splits {
    // lines[i] uses new_endpts[i] and [i + 1]
    pub lines: Vec<LineString>,
    pub new_endpts: Vec<Point>,
}

pub fn split_center(graph: &Graph, center_line: &LineString, face: &Face) -> Splits {
    // Where does this new center need to be split, based on connecting edges? Side roads will be
    // properly in the middle, and connections (maybe resolved by first merging the faces) will be
    // at 0 or 1.
    let mut split_fractions = Vec::new();
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
    let new_endpts = linestring_endpoints(&lines);

    Splits { lines, new_endpts }
}

fn linestring_endpoints(lines: &Vec<LineString>) -> Vec<Point> {
    let mut pts = Vec::new();
    for line in lines {
        pts.push(line.0[0].into());
    }
    let last_line = lines.last().unwrap();
    pts.push(last_line.0[last_line.0.len() - 1].into());
    pts
}
