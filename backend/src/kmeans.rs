// TODO Find an existing crate that doesn't pull in so many dependencies.

use geo::Coord;

pub fn kmeans_2(points: Vec<Coord>, max_iters: usize) -> Vec<usize> {
    // TODO Start from lowest and highest bearing?
    let mut center1 = points[0];
    let mut center2 = points[1];
    let mut labels = vec![0; points.len()];

    for _ in 0..max_iters {
        let mut cluster1 = Vec::new();
        let mut cluster2 = Vec::new();

        for (idx, pt) in points.iter().enumerate() {
            if euclidean(*pt, center1) <= euclidean(*pt, center2) {
                labels[idx] = 0;
                cluster1.push(*pt);
            } else {
                labels[idx] = 1;
                cluster2.push(*pt);
            }
        }

        if !cluster1.is_empty() {
            center1 = mean(&cluster1);
        }
        if !cluster2.is_empty() {
            center2 = mean(&cluster2);
        }
    }

    labels
}

// TODO Use the trait
fn euclidean(a: Coord, b: Coord) -> f64 {
    (a.x - b.x).powi(2) + (a.y - b.y).powi(2)
}

fn mean(points: &[Coord]) -> Coord {
    let n = points.len() as f64;
    let sum_x: f64 = points.iter().map(|p| p.x).sum();
    let sum_y: f64 = points.iter().map(|p| p.y).sum();
    Coord {
        x: sum_x / n,
        y: sum_y / n,
    }
}
