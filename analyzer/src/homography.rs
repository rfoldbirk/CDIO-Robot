//! Pure-Rust ground-plane homography. std-only, NO OpenCV.
//!
//! Maps image pixels of points ON THE ARENA FLOOR to a rectified (un-warped)
//! ground plane, removing the oblique camera's perspective/parallax for any
//! point that actually lies on the floor (balls, walls, the cross). The two
//! robot markers are ELEVATED, so after rectification they carry a residual
//! parallax that `correct_marker` can optionally remove (DEFAULT OFF: k==1.0).
//!
//! Everything here is std-only and unit-testable without OpenCV/cargo.
#![allow(dead_code)]

#[derive(Clone, Copy, Debug)]
pub struct Homography {
    pub m: [[f64; 3]; 3],
}

/// Solve the 8x8 linear system A h = b for the 8 unknown homography entries
/// (h33 fixed to 1) via Gaussian elimination with partial pivoting. Returns
/// None on a singular/near-singular system (collinear/degenerate corners) or
/// any non-finite pivot — NEVER panics, NEVER divides by a ~0 pivot.
fn solve8(a: &mut [[f64; 8]; 8], b: &mut [f64; 8]) -> Option<[f64; 8]> {
    const EPS: f64 = 1e-9;
    for col in 0..8 {
        let mut piv = col;
        let mut best = a[col][col].abs();
        for r in (col + 1)..8 {
            if a[r][col].abs() > best {
                best = a[r][col].abs();
                piv = r;
            }
        }
        if best < EPS {
            return None;
        }
        if piv != col {
            a.swap(piv, col);
            b.swap(piv, col);
        }
        let d = a[col][col];
        for r in (col + 1)..8 {
            let f = a[r][col] / d;
            if f != 0.0 {
                for k in col..8 {
                    a[r][k] -= f * a[col][k];
                }
                b[r] -= f * b[col];
            }
        }
    }
    let mut h = [0.0f64; 8];
    for i in (0..8).rev() {
        let mut s = b[i];
        for k in (i + 1)..8 {
            s -= a[i][k] * h[k];
        }
        if a[i][i].abs() < EPS {
            return None;
        }
        h[i] = s / a[i][i];
        if !h[i].is_finite() {
            return None;
        }
    }
    Some(h)
}

impl Homography {
    /// Build the 3x3 homography mapping the 4 `src` points to the 4 `dst`
    /// points (getPerspectiveTransform-equivalent). Returns None on a singular
    /// solve or any non-finite entry.
    pub fn from_corners(src: [(f64, f64); 4], dst: [(f64, f64); 4]) -> Option<Homography> {
        let mut a = [[0.0f64; 8]; 8];
        let mut b = [0.0f64; 8];
        for i in 0..4 {
            let (x, y) = src[i];
            let (u, v) = dst[i];
            let r = 2 * i;
            a[r] = [x, y, 1.0, 0.0, 0.0, 0.0, -x * u, -y * u];
            b[r] = u;
            a[r + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -x * v, -y * v];
            b[r + 1] = v;
        }
        let h = solve8(&mut a, &mut b)?;
        let m = [[h[0], h[1], h[2]], [h[3], h[4], h[5]], [h[6], h[7], 1.0]];
        for row in &m {
            for c in row {
                if !c.is_finite() {
                    return None;
                }
            }
        }
        Some(Homography { m })
    }

    /// Apply H to an image pixel (x,y): H * [x,y,1]^T then perspective divide.
    /// Returns None if the homogeneous w is non-finite or ~0 (point at/near the
    /// horizon line) or either output coordinate is non-finite.
    pub fn apply(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let m = &self.m;
        let w = m[2][0] * x + m[2][1] * y + m[2][2];
        if !w.is_finite() || w.abs() < 1e-12 {
            return None;
        }
        let u = (m[0][0] * x + m[0][1] * y + m[0][2]) / w;
        let v = (m[1][0] * x + m[1][1] * y + m[1][2]) / w;
        if !u.is_finite() || !v.is_finite() {
            return None;
        }
        Some((u, v))
    }

    /// Closed-form 3x3 inverse via adjugate / determinant. Returns None on a
    /// non-finite or ~0 determinant, or any non-finite entry.
    pub fn inverse(&self) -> Option<Homography> {
        let m = &self.m;
        let c00 = m[1][1] * m[2][2] - m[1][2] * m[2][1];
        let c01 = m[1][2] * m[2][0] - m[1][0] * m[2][2];
        let c02 = m[1][0] * m[2][1] - m[1][1] * m[2][0];
        let det = m[0][0] * c00 + m[0][1] * c01 + m[0][2] * c02;
        if !det.is_finite() || det.abs() < 1e-12 {
            return None;
        }
        let inv = 1.0 / det;
        let r = [
            [c00 * inv, (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv, (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv],
            [c01 * inv, (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv, (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv],
            [c02 * inv, (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv, (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv],
        ];
        for row in &r {
            for c in row {
                if !c.is_finite() {
                    return None;
                }
            }
        }
        Some(Homography { m: r })
    }
}

/// Twice the signed area (shoelace) of the quad walked TL -> TR -> BR -> BL.
/// The input order is [TL, TR, BL, BR]. The sign encodes the winding; the
/// magnitude is twice the enclosed polygon area. Used to reject a
/// mislabeled/flipped/degenerate corner set BEFORE solving, because a flipped
/// quad still solves to a finite-but-wrong H that no None-guard would catch.
fn signed_area_tl_tr_br_bl(c: &[(f64, f64); 4]) -> f64 {
    let tl = c[0];
    let tr = c[1];
    let bl = c[2];
    let br = c[3];
    let poly = [tl, tr, br, bl];
    let mut s = 0.0;
    for i in 0..4 {
        let (x1, y1) = poly[i];
        let (x2, y2) = poly[(i + 1) % 4];
        s += x1 * y2 - x2 * y1;
    }
    s * 0.5
}

/// Convenience: build the ground homography that rectifies the 4 field corners
/// (image-pixel TL,TR,BL,BR) to their OWN axis-aligned bounding-box rectangle
/// (min/max of the 4). No metric field dimensions needed, and rectified coords
/// stay near the current pixel scale so existing thresholds remain ballpark.
///
/// Guards (all return None -> caller falls back to pixel space):
///   * any non-finite corner;
///   * bounding-box extent below MIN_EXTENT in x or y (degenerate/tiny);
///   * WRONG WINDING: in image coords y grows DOWNWARD, so a correctly-labeled
///     quad walked TL,TR,BR,BL has POSITIVE shoelace area. A flipped/swapped or
///     mislabeled set yields <=0 area -> rejected. This is the one failure mode
///     the 8x8 solve's None-guards cannot catch (it solves fine, just wrong).
///   * SLIM/SHEARED: enclosed area < 25% of the bounding-box area -> rejected,
///     so a sliver or sentinel-laced quad cannot build a wildly-scaled H.
pub fn from_field_corners(corners: [(f64, f64); 4]) -> Option<Homography> {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for &(x, y) in &corners {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        if x < min_x { min_x = x; }
        if x > max_x { max_x = x; }
        if y < min_y { min_y = y; }
        if y > max_y { max_y = y; }
    }
    const MIN_EXTENT: f64 = 10.0;
    let ext_x = max_x - min_x;
    let ext_y = max_y - min_y;
    if ext_x < MIN_EXTENT || ext_y < MIN_EXTENT {
        return None;
    }

    // ORIENTATION / WINDING GUARD: see doc comment above. Catches the
    // finite-but-wrong H before it is built and cached.
    let area = signed_area_tl_tr_br_bl(&corners);
    let bbox_area = ext_x * ext_y;
    if !area.is_finite() || area <= 0.0 {
        return None; // wrong winding (flipped/mislabeled) or degenerate
    }
    if area < 0.25 * bbox_area {
        return None; // too slim / not a real rectangle-ish quad
    }

    let dst = [
        (min_x, min_y),
        (max_x, min_y),
        (min_x, max_y),
        (max_x, max_y),
    ];
    Homography::from_corners(corners, dst)
}

/// Optional ROBOT-MARKER HEIGHT CORRECTION (DEFAULT OFF when k == 1.0).
///
/// An elevated marker's rectified position is displaced AWAY from the camera
/// nadir by perspective. Pull it back toward `nadir` by k = (H_cam - h)/H_cam,
/// where H_cam is the camera height above the floor and h the marker height.
///   corrected = nadir + (apparent - nadir) * k
/// k == 1.0 is the identity fast-path (NO correction) and is the default, so
/// out-of-the-box behavior is ground-homography-only (needs zero measurements).
pub fn correct_marker(p: (f64, f64), nadir: (f64, f64), k: f64) -> (f64, f64) {
    if k == 1.0 {
        return p;
    }
    (nadir.0 + (p.0 - nadir.0) * k, nadir.1 + (p.1 - nadir.1) * k)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn corners_map_exactly() {
        let src = [(10.0, 20.0), (210.0, 40.0), (30.0, 250.0), (240.0, 260.0)];
        let dst = [(10.0, 20.0), (240.0, 20.0), (10.0, 260.0), (240.0, 260.0)];
        let h = Homography::from_corners(src, dst).unwrap();
        for i in 0..4 {
            let (u, v) = h.apply(src[i].0, src[i].1).unwrap();
            assert!(approx(u, dst[i].0, 1e-6) && approx(v, dst[i].1, 1e-6));
        }
    }

    #[test]
    fn round_trip_interior() {
        let src = [(10.0, 20.0), (210.0, 40.0), (30.0, 250.0), (240.0, 260.0)];
        let dst = [(10.0, 20.0), (240.0, 20.0), (10.0, 260.0), (240.0, 260.0)];
        let h = Homography::from_corners(src, dst).unwrap();
        let p = (123.4, 88.7);
        let f = h.apply(p.0, p.1).unwrap();
        let inv = h.inverse().unwrap();
        let r = inv.apply(f.0, f.1).unwrap();
        assert!(approx(r.0, p.0, 1e-6) && approx(r.1, p.1, 1e-6));
    }

    #[test]
    fn identity_maps_self() {
        let sq = [(0.0, 0.0), (100.0, 0.0), (0.0, 100.0), (100.0, 100.0)];
        let h = Homography::from_corners(sq, sq).unwrap();
        for &(x, y) in &[(0.0, 0.0), (50.0, 50.0), (12.3, 87.6)] {
            let (u, v) = h.apply(x, y).unwrap();
            assert!(approx(u, x, 1e-9) && approx(v, y, 1e-9));
        }
    }

    #[test]
    fn collinear_is_none() {
        let coll = [(0.0, 0.0), (10.0, 10.0), (20.0, 20.0), (30.0, 30.0)];
        let dst = [(10.0, 20.0), (240.0, 20.0), (10.0, 260.0), (240.0, 260.0)];
        assert!(Homography::from_corners(coll, dst).is_none());
    }

    #[test]
    fn w_zero_apply_none() {
        let degen = Homography { m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]] };
        assert!(degen.apply(5.0, 5.0).is_none());
    }

    #[test]
    fn marker_correction() {
        assert_eq!(correct_marker((50.0, 60.0), (100.0, 100.0), 1.0), (50.0, 60.0));
        let c = correct_marker((0.0, 0.0), (100.0, 100.0), 0.5);
        assert!(approx(c.0, 50.0, 1e-12) && approx(c.1, 50.0, 1e-12));
    }

    #[test]
    fn field_corners_builds_and_degenerate_none() {
        // realistic image-coord quad: TL,TR,BL,BR with y DOWN (clockwise).
        let corners = [(10.0, 20.0), (210.0, 40.0), (30.0, 250.0), (240.0, 260.0)];
        assert!(from_field_corners(corners).is_some());
        let flat = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        assert!(from_field_corners(flat).is_none());
    }

    #[test]
    fn field_corners_rectifies_interior_to_correct_side() {
        // A real-ish image quad in pixel coords (y DOWN), order [TL,TR,BL,BR].
        let corners = [(50.0, 40.0), (300.0, 60.0), (30.0, 280.0), (330.0, 300.0)];
        let h = from_field_corners(corners).expect("valid quad builds");
        // Each input corner must rectify to its bounding-box corner: pins that
        // TL->(minx,miny), TR->(maxx,miny), BL->(minx,maxy), BR->(maxx,maxy),
        // i.e. NOT flipped. min_x=30,max_x=330,min_y=40,max_y=300.
        let tl = h.apply(50.0, 40.0).unwrap();
        let tr = h.apply(300.0, 60.0).unwrap();
        let bl = h.apply(30.0, 280.0).unwrap();
        let br = h.apply(330.0, 300.0).unwrap();
        assert!(approx(tl.0, 30.0, 1e-4) && approx(tl.1, 40.0, 1e-4), "tl {:?}", tl);
        assert!(approx(tr.0, 330.0, 1e-4) && approx(tr.1, 40.0, 1e-4), "tr {:?}", tr);
        assert!(approx(bl.0, 30.0, 1e-4) && approx(bl.1, 300.0, 1e-4), "bl {:?}", bl);
        assert!(approx(br.0, 330.0, 1e-4) && approx(br.1, 300.0, 1e-4), "br {:?}", br);
        // interior point stays interior (left of center maps left of center).
        let mid = h.apply(180.0, 170.0).unwrap();
        assert!(mid.0 > 30.0 && mid.0 < 330.0 && mid.1 > 40.0 && mid.1 < 300.0);
    }

    #[test]
    fn flipped_winding_is_rejected() {
        // Swap TR and BL labels of a valid quad -> wrong winding -> must be None,
        // proving a mislabeled corner set does NOT silently build a flipped H.
        let good = [(50.0, 40.0), (300.0, 60.0), (30.0, 280.0), (330.0, 300.0)];
        assert!(from_field_corners(good).is_some());
        let swapped = [good[0], good[2], good[1], good[3]]; // TL, BL, TR, BR
        assert!(from_field_corners(swapped).is_none());
    }

    #[test]
    fn slim_quad_is_rejected() {
        // Non-collinear but very slim quad (a sliver): bbox is 300x300 but the
        // enclosed area is tiny, so area < 25% of bbox -> rejected even though the
        // 8x8 solve itself would succeed.
        let needle = [(0.0, 0.0), (20.0, 0.0), (280.0, 300.0), (300.0, 300.0)];
        // bbox 300x300 = 90000; enclosed area ~6000 (ratio ~0.067 < 0.25).
        assert!(from_field_corners(needle).is_none());
    }
}
