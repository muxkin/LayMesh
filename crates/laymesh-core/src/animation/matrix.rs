//! CSS transform-list fallback: decompose, interpolate and recompose a 4x4
//! matrix. Matching transform functions are interpolated before this fallback.
use super::values::{Matrix, identity, multiply, translate};
pub fn inverse(m: Matrix) -> Option<Matrix> {
    let mut a = [[0.; 8]; 4];
    for r in 0..4 {
        for c in 0..4 {
            a[r][c] = m[r * 4 + c];
        }
        a[r][r + 4] = 1.;
    }
    for i in 0..4 {
        let pivot = (i..4).max_by(|a0, b0| a[*a0][i].abs().total_cmp(&a[*b0][i].abs()))?;
        a.swap(i, pivot);
        let n = a[i][i];
        if n.abs() < 1e-15 {
            return None;
        }
        for c in 0..8 {
            a[i][c] /= n;
        }
        for r in 0..4 {
            if r != i {
                let n = a[r][i];
                for c in 0..8 {
                    a[r][c] -= n * a[i][c];
                }
            }
        }
    }
    let mut out = [0.; 16];
    for r in 0..4 {
        for c in 0..4 {
            out[r * 4 + c] = a[r][c + 4];
        }
    }
    Some(out)
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn mul(a: [f64; 3], s: f64) -> [f64; 3] {
    a.map(|v| v * s)
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
struct Parts {
    translation: [f64; 3],
    scale: [f64; 3],
    skew: [f64; 3],
    perspective: [f64; 4],
    quaternion: [f64; 4],
}
fn decompose(mut m: Matrix) -> Option<Parts> {
    if m[15].abs() < 1e-15 {
        return None;
    }
    let norm = m[15];
    for v in &mut m {
        *v /= norm;
    }
    let mut pm = m;
    pm[12] = 0.;
    pm[13] = 0.;
    pm[14] = 0.;
    pm[15] = 1.;
    let inv = inverse(pm)?;
    let rhs = [m[12], m[13], m[14], m[15]];
    let mut perspective = [0.; 4];
    for r in 0..4 {
        for c in 0..4 {
            perspective[r] += inv[c * 4 + r] * rhs[c];
        }
    }
    let translation = [m[3], m[7], m[11]];
    let mut cols = [[m[0], m[4], m[8]], [m[1], m[5], m[9]], [m[2], m[6], m[10]]];
    let mut scale = [0.; 3];
    let mut skew = [0.; 3];
    scale[0] = dot(cols[0], cols[0]).sqrt();
    if scale[0] < 1e-15 {
        return None;
    }
    cols[0] = mul(cols[0], 1. / scale[0]);
    skew[0] = dot(cols[0], cols[1]);
    cols[1] = sub(cols[1], mul(cols[0], skew[0]));
    scale[1] = dot(cols[1], cols[1]).sqrt();
    if scale[1] < 1e-15 {
        return None;
    }
    cols[1] = mul(cols[1], 1. / scale[1]);
    skew[0] /= scale[1];
    skew[1] = dot(cols[0], cols[2]);
    cols[2] = sub(cols[2], mul(cols[0], skew[1]));
    skew[2] = dot(cols[1], cols[2]);
    cols[2] = sub(cols[2], mul(cols[1], skew[2]));
    scale[2] = dot(cols[2], cols[2]).sqrt();
    if scale[2] < 1e-15 {
        return None;
    }
    cols[2] = mul(cols[2], 1. / scale[2]);
    skew[1] /= scale[2];
    skew[2] /= scale[2];
    if dot(cols[0], cross(cols[1], cols[2])) < 0. {
        for i in 0..3 {
            scale[i] *= -1.;
            cols[i] = mul(cols[i], -1.);
        }
    }
    let r = |row: usize, col: usize| cols[col][row];
    let trace = r(0, 0) + r(1, 1) + r(2, 2);
    let q;
    if trace > 0. {
        let s = (trace + 1.).sqrt() * 2.;
        q = [
            (r(2, 1) - r(1, 2)) / s,
            (r(0, 2) - r(2, 0)) / s,
            (r(1, 0) - r(0, 1)) / s,
            s / 4.,
        ];
    } else {
        let i = if r(0, 0) > r(1, 1) && r(0, 0) > r(2, 2) {
            0
        } else if r(1, 1) > r(2, 2) {
            1
        } else {
            2
        };
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        let s = (1. + r(i, i) - r(j, j) - r(k, k)).max(0.).sqrt() * 2.;
        if s < 1e-15 {
            return None;
        }
        let mut a = [0.; 4];
        a[i] = s / 4.;
        a[j] = (r(j, i) + r(i, j)) / s;
        a[k] = (r(k, i) + r(i, k)) / s;
        a[3] = (r(k, j) - r(j, k)) / s;
        q = a;
    }
    Some(Parts {
        translation,
        scale,
        skew,
        perspective,
        quaternion: q,
    })
}
pub fn interpolate(a: Matrix, b: Matrix, p: f64) -> Matrix {
    let (Some(a), Some(b)) = (decompose(a), decompose(b)) else {
        return if p < 0.5 { a } else { b };
    };
    let lerp = |a: f64, b: f64| a + (b - a) * p;
    let mut q = b.quaternion;
    let mut dot = a.quaternion.iter().zip(q).map(|(a, b)| a * b).sum::<f64>();
    if dot < 0. {
        dot = -dot;
        q = q.map(|x| -x);
    }
    let (u, v) = if dot > 0.9995 {
        (1. - p, p)
    } else {
        let t = dot.clamp(-1., 1.).acos();
        (((1. - p) * t).sin() / t.sin(), (p * t).sin() / t.sin())
    };
    let mut q = std::array::from_fn::<_, 4, _>(|i| a.quaternion[i] * u + q[i] * v);
    let n = q.iter().map(|v| v * v).sum::<f64>().sqrt();
    for v in &mut q {
        *v /= n;
    }
    let [x, y, z, w] = q;
    let mut r = identity();
    r[0] = 1. - 2. * (y * y + z * z);
    r[1] = 2. * (x * y - z * w);
    r[2] = 2. * (x * z + y * w);
    r[4] = 2. * (x * y + z * w);
    r[5] = 1. - 2. * (x * x + z * z);
    r[6] = 2. * (y * z - x * w);
    r[8] = 2. * (x * z - y * w);
    r[9] = 2. * (y * z + x * w);
    r[10] = 1. - 2. * (x * x + y * y);
    let mut perspective = identity();
    for i in 0..4 {
        perspective[12 + i] = lerp(a.perspective[i], b.perspective[i]);
    }
    let t = std::array::from_fn::<_, 3, _>(|i| lerp(a.translation[i], b.translation[i]));
    let mut skew = identity();
    skew[1] = lerp(a.skew[0], b.skew[0]);
    skew[2] = lerp(a.skew[1], b.skew[1]);
    skew[6] = lerp(a.skew[2], b.skew[2]);
    let mut scale = identity();
    for i in 0..3 {
        scale[i * 5] = lerp(a.scale[i], b.scale[i]);
    }
    multiply(
        multiply(
            multiply(multiply(perspective, translate(t[0], t[1], t[2])), r),
            skew,
        ),
        scale,
    )
}
