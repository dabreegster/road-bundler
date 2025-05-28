use geo::{Closest, ClosestPoint, Coord, Euclidean, InterpolatableLine, Length, LineString};

pub fn average_linestrings(ls1: &LineString, ls2: &LineString) -> Option<LineString> {
    // Walk along ls1 at this step size, find the closest equivalent point in ls2, and average the
    // two.
    let step_size = 10.0;

    let ls1_length = Euclidean.length(ls1);
    let mut pts = Vec::new();

    let mut distance = 0.0;
    loop {
        let pt1 = ls1
            .point_at_distance_from_start(&Euclidean, distance)
            .expect("point_at_distance_from_start for ls1 should work");
        // Even if ls1 and ls2 point in opposite directions (usually true), the closest equivalent
        // point will be fine.
        if let Some(pt2) = match ls2.closest_point(&pt1) {
            Closest::Intersection(pt) => Some(pt),
            Closest::SinglePoint(pt) => Some(pt),
            Closest::Indeterminate => {
                error!("closest_point on ls2 at distance {distance} is Indeterminate");
                None
            }
        } {
            pts.push(Coord {
                x: (pt1.x() + pt2.x()) / 2.0,
                y: (pt1.y() + pt2.y()) / 2.0,
            });
        }

        if distance == ls1_length {
            break;
        }

        distance += step_size;
        // Always include the very last point
        if distance > ls1_length {
            distance = ls1_length;
        }
    }

    if pts.len() < 2 {
        error!("step_size too big for ls1_length {ls1_length}");
        return None;
    }

    Some(LineString::new(pts))
}
