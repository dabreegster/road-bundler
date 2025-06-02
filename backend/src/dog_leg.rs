use geo::{Euclidean, InterpolatableLine, Length};

use crate::geo_helpers::linestring_bearing;
use crate::{EdgeID, Intersection, IntersectionProvenance, RoadBundler};

pub struct DogLeg {
    // In no particular order
    side_roads: (EdgeID, EdgeID),
}

impl RoadBundler {
    pub fn collapse_edge(&mut self, collapse_e: EdgeID) {
        let (src, dst, midpt) = {
            let edge = &self.graph.edges[&collapse_e];
            let midpt = edge
                .linestring
                .point_at_ratio_from_start(&Euclidean, 0.5)
                .unwrap();
            (edge.src, edge.dst, midpt)
        };
        let dog_leg = self.is_dog_leg(collapse_e);

        self.graph.remove_edge(collapse_e);

        // Create a new intersection at the middle of the short edge
        let new_intersection = self.graph.new_intersection_id();
        self.graph.intersections.insert(
            new_intersection,
            Intersection {
                id: new_intersection,
                edges: Vec::new(),
                point: midpt,
                provenance: IntersectionProvenance::Synthetic,
            },
        );

        // Remove the two old intersections, reconnecting the edges
        let extend_geometry = dog_leg.is_none();
        self.graph
            .replace_intersection(src, new_intersection, extend_geometry);
        self.graph
            .replace_intersection(dst, new_intersection, extend_geometry);

        // If it's a dog leg, fix up the geometry. Extend the two "main roads" up to the new
        // intersection. Leave the two "side roads" alone -- their linestring will not touch the
        // new intersection.
        if let Some(dog_leg) = dog_leg {
            for e in &self.graph.intersections[&new_intersection].edges {
                let fix_edge = self.graph.edges.get_mut(e).unwrap();
                if *e == dog_leg.side_roads.0 || *e == dog_leg.side_roads.1 {
                    // Trim off the first or last meter, then connect to the new intersection
                    if fix_edge.src == new_intersection {
                        if let Some(trim_pt) = fix_edge
                            .linestring
                            .point_at_distance_from_start(&Euclidean, 1.0)
                        {
                            fix_edge.linestring.0[0] = trim_pt.into();
                            fix_edge.linestring.0.insert(0, midpt.into());
                        }
                    } else {
                        if let Some(trim_pt) = fix_edge
                            .linestring
                            .point_at_distance_from_end(&Euclidean, 1.0)
                        {
                            fix_edge.linestring.0.pop();
                            fix_edge.linestring.0.push(trim_pt.into());
                            fix_edge.linestring.0.push(midpt.into());
                        }
                    }
                } else {
                    // Extend the main roads up to the new point
                    if fix_edge.src == new_intersection {
                        fix_edge.linestring.0.insert(0, midpt.into());
                    } else {
                        fix_edge.linestring.0.push(midpt.into());
                    }
                }
            }
        }
    }

    pub fn is_dog_leg(&self, e: EdgeID) -> Option<DogLeg> {
        let edge = &self.graph.edges[&e];
        if Euclidean.length(&edge.linestring) > 5.0 {
            return None;
        }
        let mut src_edges = self.graph.intersections[&edge.src].edges.clone();
        let mut dst_edges = self.graph.intersections[&edge.dst].edges.clone();
        if src_edges.len() != 3 || dst_edges.len() != 3 {
            return None;
        }

        // Find the two "side roads" with a different name than the short edge
        // (We could use angle to be safer than name)
        src_edges.retain(|x| self.graph.edges[x].get_name() != edge.get_name());
        dst_edges.retain(|x| self.graph.edges[x].get_name() != edge.get_name());

        if src_edges.len() != 1 || dst_edges.len() != 1 {
            return None;
        }

        let src_edge = &self.graph.edges[&src_edges[0]];
        let dst_edge = &self.graph.edges[&dst_edges[0]];

        // Orient each linestring to point towards the intersection on edge
        let mut src_ls = src_edge.linestring.clone();
        let mut dst_ls = dst_edge.linestring.clone();
        if src_edge.dst != edge.src {
            src_ls.0.reverse();
        }
        if dst_edge.dst != edge.dst {
            dst_ls.0.reverse();
        }
        let b1 = linestring_bearing(&src_ls);
        let b2 = linestring_bearing(&dst_ls);

        // If these both point about the same way, they're on the "same side" of the short edge.
        // Not a dog leg.
        if (b1 - b2).abs() < 30.0 {
            return None;
        }

        Some(DogLeg {
            side_roads: (src_edges[0], dst_edges[0]),
        })
    }
}
