//! Exact-order nearest boundary queries over flattened contours.
//!
//! The original scan visits (contour A, contour B, segment A, segment B,
//! endpoint direction) in that order and updates only for a 1e-12 improvement.
//! A BVH excludes segment pairs whose bounding boxes cannot improve the answer;
//! candidates are restored to their original order before projection, retaining
//! both the floating-point calculation and the original tie behavior.
type Point = [f64; 2];
type Bounds = [f64; 4];
type Nearest = (Point, Point, f64);
const LEAF: usize = 8;
fn segment_bounds(a: Point, b: Point) -> Bounds {
    [
        a[0].min(b[0]),
        a[1].min(b[1]),
        a[0].max(b[0]),
        a[1].max(b[1]),
    ]
}
fn union(a: Bounds, b: Bounds) -> Bounds {
    [
        a[0].min(b[0]),
        a[1].min(b[1]),
        a[2].max(b[2]),
        a[3].max(b[3]),
    ]
}
fn could_improve(a: Bounds, b: Bounds, distance: f64) -> bool {
    let dx = (a[0] - b[2]).max(b[0] - a[2]).max(0.);
    let dy = (a[1] - b[3]).max(b[1] - a[3]).max(0.);
    // Roundoff in a projected endpoint must not make its distance appear just
    // below the theoretical box bound, particularly for translated coordinates.
    let scale = a.into_iter().chain(b).map(f64::abs).fold(1., f64::max);
    dx.hypot(dy) <= distance + 16. * f64::EPSILON * scale
}
struct Node {
    bounds: Bounds,
    start: usize,
    end: usize,
    children: Option<[usize; 2]>,
}
struct Segments {
    indices: Vec<usize>,
    nodes: Vec<Node>,
}
impl Segments {
    fn new(contour: &[Point]) -> Self {
        let bounds: Vec<_> = (0..contour.len())
            .map(|i| segment_bounds(contour[i], contour[(i + 1) % contour.len()]))
            .collect();
        let mut tree = Self {
            indices: (0..contour.len()).collect(),
            nodes: vec![],
        };
        if !contour.is_empty() {
            tree.build(&bounds, 0, contour.len());
        }
        tree
    }
    fn build(&mut self, bounds: &[Bounds], start: usize, end: usize) -> usize {
        let bbox = self.indices[start..end]
            .iter()
            .skip(1)
            .fold(bounds[self.indices[start]], |a, &i| union(a, bounds[i]));
        let node = self.nodes.len();
        self.nodes.push(Node {
            bounds: bbox,
            start,
            end,
            children: None,
        });
        if end - start > LEAF {
            let axis = usize::from(bbox[3] - bbox[1] > bbox[2] - bbox[0]);
            let midpoint = (end - start) / 2;
            self.indices[start..end].select_nth_unstable_by(midpoint, |&a, &b| {
                (bounds[a][axis] + bounds[a][axis + 2])
                    .total_cmp(&(bounds[b][axis] + bounds[b][axis + 2]))
                    .then(a.cmp(&b))
            });
            let middle = start + midpoint;
            let left = self.build(bounds, start, middle);
            let right = self.build(bounds, middle, end);
            self.nodes[node].children = Some([left, right]);
        }
        node
    }
    fn candidates(&self, node: usize, bounds: Bounds, distance: f64, out: &mut Vec<usize>) {
        let node = &self.nodes[node];
        if !could_improve(bounds, node.bounds, distance) {
            return;
        }
        if let Some([left, right]) = node.children {
            self.candidates(left, bounds, distance, out);
            self.candidates(right, bounds, distance, out);
        } else {
            out.extend_from_slice(&self.indices[node.start..node.end]);
        }
    }
}
fn project_pair(a: Point, aa: Point, b: Point, bb: Point, best: &mut Nearest) {
    for (p, s, t, swap) in [(a, b, bb, false), (b, a, aa, true)] {
        let dx = t[0] - s[0];
        let dy = t[1] - s[1];
        let u = (((p[0] - s[0]) * dx + (p[1] - s[1]) * dy) / (dx * dx + dy * dy).max(1e-30))
            .clamp(0., 1.);
        let q = [s[0] + u * dx, s[1] + u * dy];
        let d = (p[0] - q[0]).hypot(p[1] - q[1]);
        if d < best.2 - 1e-12 {
            *best = if swap { (q, p, d) } else { (p, q, d) };
        }
    }
}
pub(super) fn boundaries(a: &[Vec<Point>], b: &[Vec<Point>]) -> Nearest {
    let trees: Vec<_> = b.iter().map(|contour| Segments::new(contour)).collect();
    let mut best = ([0., 0.], [0., 0.], f64::INFINITY);
    let mut candidates = Vec::new();
    for ca in a {
        for (cb, tree) in b.iter().zip(&trees) {
            if cb.is_empty() {
                continue;
            }
            for ai in 0..ca.len() {
                let (x, xa) = (ca[ai], ca[(ai + 1) % ca.len()]);
                candidates.clear();
                tree.candidates(0, segment_bounds(x, xa), best.2, &mut candidates);
                candidates.sort_unstable();
                for &bi in &candidates {
                    project_pair(x, xa, cb[bi], cb[(bi + 1) % cb.len()], &mut best);
                }
            }
        }
    }
    best
}
#[cfg(test)]
mod tests {
    use super::*;
    fn exhaustive(a: &[Vec<Point>], b: &[Vec<Point>]) -> Nearest {
        let mut best = ([0., 0.], [0., 0.], f64::INFINITY);
        for ca in a {
            for cb in b {
                for ai in 0..ca.len() {
                    for bi in 0..cb.len() {
                        project_pair(
                            ca[ai],
                            ca[(ai + 1) % ca.len()],
                            cb[bi],
                            cb[(bi + 1) % cb.len()],
                            &mut best,
                        );
                    }
                }
            }
        }
        best
    }
    #[test]
    fn bvh_preserves_scan_order_ties_degenerate_segments_and_epsilon_updates() {
        let a = vec![
            vec![[0., 0.], [0., 0.], [0., 10.], [2., 10.]],
            vec![[3., 3.]],
        ];
        let b = vec![
            vec![[5., 0.], [5., 1.], [5., 5.], [5., 10.]],
            vec![[5. - 0.5e-12, 3.], [5. - 1.5e-12, 3.]],
        ];
        assert_eq!(boundaries(&a, &b), exhaustive(&a, &b));
        let mut seed = 739_u64;
        for trial in 0..200 {
            let mut point = || {
                let mut coordinate = || {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    ((seed >> 32) % 10000) as f64 / 100.
                };
                [coordinate(), coordinate()]
            };
            let a = vec![
                (0..(7 + trial % 31)).map(|_| point()).collect::<Vec<_>>(),
                vec![point()],
            ];
            let b = vec![
                (0..(19 + trial % 47)).map(|_| point()).collect::<Vec<_>>(),
                vec![point()],
            ];
            assert_eq!(boundaries(&a, &b), exhaustive(&a, &b), "trial {trial}");
        }
    }
    #[test]
    fn spatially_separated_curves_keep_exact_endpoints_at_large_offsets() {
        for offset in [0., 1e6, -1e9] {
            let curve = |shift: f64| {
                (0..512)
                    .map(|i| {
                        let theta = i as f64 * std::f64::consts::TAU / 512.;
                        [
                            offset + shift + 10. * theta.cos(),
                            offset + 10. * theta.sin(),
                        ]
                    })
                    .collect::<Vec<_>>()
            };
            let a = vec![curve(0.)];
            let b = vec![curve(40.)];
            assert_eq!(boundaries(&a, &b), exhaustive(&a, &b));
            assert!((boundaries(&a, &b).2 - 20.).abs() < 1e-8);
        }
    }
    #[test]
    #[ignore = "manual exhaustive-versus-BVH microbenchmark"]
    fn nearest_boundary_benchmark() {
        let curve = |shift: f64| {
            (0..4096)
                .map(|i| {
                    let theta = i as f64 * std::f64::consts::TAU / 4096.;
                    [shift + 10. * theta.cos(), 10. * theta.sin()]
                })
                .collect::<Vec<_>>()
        };
        let (a, b) = (vec![curve(0.)], vec![curve(40.)]);
        let t = std::time::Instant::now();
        let old = exhaustive(&a, &b);
        let old_time = t.elapsed();
        let t = std::time::Instant::now();
        let new = boundaries(&a, &b);
        let new_time = t.elapsed();
        assert_eq!(old, new);
        eprintln!("4096x4096 segments: exhaustive={old_time:?}, BVH={new_time:?}");
    }
}
