# Changes — Ball-pickup precision, wall/cross clearance & fail-soft routing

> Branch: `color-calibration`. Scope: the live robot (`analyzer` crate `track-analyzer`) plus the mirrored offline planner (`path` crate). No behavioural change to `track`, `yolo`, or `control-panel`.
>
> Three things were addressed:
> 1. **Goal 1 — close-range ball precision** so the suction reliably grabs the ball.
> 2. **Goal 2 — keep more distance from the red walls and the central cross** while still being able to reach balls near walls.
> 3. **Robustness follow-up — stop the vision/route loop from panicking** ("robot doesn't exist") on a frame that drops an obstacle blob.

All line numbers below are on `color-calibration` after these edits; they will drift, so each change also names its function.

---

## 1. Goal 1 — close-range ball precision (`analyzer/src/main.rs`)

The robot used to stop up to ~80 px short of a ball and trigger suction immediately, aiming at the **bounding-box midpoint** of the matched white pixels with a noisy car position. Four changes fix this.

### 1.1 Pixel centroid instead of AABB midpoint — `Grouping::center()` (~line 702)
`center()` now returns the **mean** of all member pixels instead of `((min+max)/2)`. The old AABB midpoint is set by just the two extreme pixels, so a single stray/outlier pixel or an asymmetric (glare-cut) blob shifted the reported center by tens of px. The mean moves by only `offset / N`. This improves **every** consumer: `get_balls()`, `get_car_center()`, `get_car_direction()`.

### 1.2 Color-aware flood fill — `group_marks()` (~line 803)
The 8-connected flood fill now only merges a neighbour that shares the **same color** as the seed pixel. Previously a white ball touching the red wall/cross merged into one blob, producing a phantom center. (Walls and the cross are both red, so they still group exactly as before — only white-vs-red adjacency changes, which is the intended fix.)

### 1.3 Magnitude-based jitter clamp (~line 412)
The old clamp used a **signed** `> 10` test: it only caught jumps toward +x/+y, used a 10 px threshold the car exceeds while driving, and on a real move it both rejected the new position **and** wrote the stale value back — freezing the car position permanently once it moved fast, which corrupted the heading vector and the route. The new clamp rejects only true outliers by **squared distance** (`JITTER_REJECT_SQ`, ~60 px) and updates `last_car_pos` **only with the accepted value**, so a genuine drive is never frozen.

### 1.4 Suction-mouth final approach (close-range control block, ~lines 481–620)
The single instantaneous SUCK was replaced by a guarded **final approach**:

- The physical **suction mouth** is modelled as a forward offset (`SUCK_OFFSET`) from the `car_center` marker along the heading; its projected position is drawn each frame as a **blue dot** for calibration.
- Within `APPROACH_DISTANCE` of the ball the robot **aims directly at the ball**, bypassing the A* waypoint and obstacle weighting — this is what lets it reach a ball ~50 px from a wall.
- It **creeps** the last leg in **pulsed** forward/stop bursts (because motor commands latch), so a single fast frame can't skip the trigger window.
- SUCK fires on an **at-or-beyond** test: the signed along-heading distance from the mouth to the ball is `<= SUCK_TOLERANCE` (allows slight overshoot) **and** the cross-track offset is `<= SUCK_LATERAL_TOLERANCE`. An overshoot latches the trigger instead of flipping the heading and oscillating.
- A **lost-direction guard** (`head_valid`) checks the real `(0,0)` sentinel that `get_car_direction()` returns when the marker is missing, so a lost heading can never trigger SUCK on a garbage direction.
- A **Run-only interlock** (`CLOSE_RADIUS`) only honours "close enough" when the straight-line car→ball distance is genuinely within range, so a capped/failed A* route (which reports `route_lenth == 0`) cannot collapse into a straight bee-line through obstacles.

The end-game state machine (`EndGoToPos → EndTurnAround → EndOpen → EndShakeItOut → End`) is preserved byte-for-byte: the new `final_approach`/creep/`on_ball` logic is **Run-only**, and the End* transitions still fire on the unchanged close condition.

---

## 2. Goal 2 — wall & cross clearance (`analyzer/src/path.rs` + mirrored to `path/src/main.rs`)

The A* soft-penalty **weights** were raised modestly while keeping the quartic exponent and the buffers (350 / 400) **unchanged**:

| Penalty | Old weight | New weight | File:line |
|---------|-----------|-----------|-----------|
| Wall | 50 000 | **70 000** | `analyzer/src/path.rs:267`, `path/src/main.rs:281` |
| Cross | 50 000 | **60 000** | `analyzer/src/path.rs:362`, `path/src/main.rs:405` |

The hard exclusion zones (`HARD_WALL_CLEARANCE` / `HARD_CROSS_CLEARANCE`) stay **commented out** — see the camera-parallax constraint in §5. The penalty remains a finite **soft** cost, so a route always exists from any start.

**Why only a modest bump?** The squared-distance heuristic + 1000-iteration cap is fragile. A simulation of the exact A* against the real scene showed that the more aggressive reshape first considered (quadratic exponent, weight 120 000, buffers 450/480) makes **every ball hit the iteration cap and stall** — the exact "no route → robot doesn't exist" failure the parallax constraint exists to avoid. The chosen 70 000/60 000 is the safe lever found by parameter sweep. See §4.

The same two literals are mirrored into the offline `path` crate so the local route-visualization harness (`map_computed.jpg`) matches the hardware.

---

## 3. Robustness follow-up — fail-soft obstacle handling

Four `.unwrap()` / index sites assumed ≥2 surviving obstacle groups (or non-empty point clouds) and **panicked the whole loop** on any frame that dropped the wall or cross blob (glare, occlusion, motion blur). Since the walls and cross are **static** in the arena, the fix caches the last good bounds and reuses them.

| Site | Was | Now |
|------|-----|-----|
| `analyzer/src/path.rs` `bounds()` (line 32) | `.unwrap()` on min/max of a possibly-empty slice | returns **`Option<Bounds>`** (`None` if empty) |
| `analyzer/src/main.rs` `get_obstacles()` (line 757) | `.first().unwrap()` / `.nth(1).unwrap()` | returns **`Option<Obstacles>`** (`None` if <2 groups) |
| `analyzer/src/main.rs` `find_cross()` (line 849) | `.nth(1).unwrap()` | `match … { None => return self }` (draws nothing) |
| `analyzer/src/main.rs` `find_map_border()` (line 893) | `.first().unwrap()` | `match … { None => return self }` (draws nothing) |

The main loop (line 442) now builds obstacle bounds into a cached `last_obst_bounds: Option<ObstacleBounds>` (declared line 285): it only updates the cache when a frame yields valid bounds, and routing (`else if let Some(obst_bounds) = &last_obst_bounds`, line 459) reuses the last good bounds. If obstacles have **never** been detected yet, routing is skipped for that frame (`println!("waiting for obstacle detection...")`) instead of crashing — self-healing the moment the border is seen once.

`find_cross()` / `find_map_border()` are visualization-only (they draw into a working image that is never displayed and feed nothing into routing), so bailing out is harmless.

---

## 4. Verification

- **Offline `path` crate compiles** (`cargo build --manifest-path path/Cargo.toml`, warnings only). The live `analyzer` crate could **not** be compiled in the dev/CI box used (OpenCV's build script needs `libclang.dylib`); the routing logic is identical to the compiled `path` crate, and the `main.rs` edits are plain `std`/`image` code. **Run `cargo check` in `analyzer/` on the OpenCV-equipped robot machine before deploying.**
- **Clearance simulation** (`path/measure_clearance.py`, a faithful Python reproduction of `calc_route` run against the real `path/{obstacles,car,balls}.json`):

  | Config | min wall | min cross | worst route-end | capped? |
  |--------|---------|-----------|-----------------|---------|
  | Baseline 50k/50k | 138 px | 17.5 px | 146 px | no |
  | Rejected quad 120k/450 | 446 px | 82.5 px | **825 px** | **YES — stall** |
  | **Chosen 70k/60k** | **150 px** | **39.4 px** | 146 px | no |

  All 4 real balls **and** 4 synthetic near-wall/corner balls are FOUND with no cap; worst route-end (146 px) stays below `APPROACH_DISTANCE` (200) so the straight final approach always reaches the ball.

---

## 5. Tunable constants (calibrate on the robot)

All in `analyzer/src/main.rs` unless noted. Defaults are reasonable starting points.

| Constant | Default | Meaning / how to calibrate |
|----------|---------|----------------------------|
| `SUCK_OFFSET` | 70.0 | px from the `car_center` marker forward to the physical mouth. **Calibrate first**: a blue dot is drawn at the projected mouth each frame — adjust until it sits on the real mouth. |
| `SUCK_TOLERANCE` | 35.0 | along-heading gap at which SUCK fires. Keep above the per-frame creep travel. |
| `SUCK_LATERAL_TOLERANCE` | 40.0 | max cross-track offset to allow SUCK (mouth half-width + slack). |
| `APPROACH_DISTANCE` | 200 | car→ball range for the straight final approach. Must stay > worst route-end (146 px) and > the 80 px early-exit. Lower toward 150 if a ball next to the **cross** gets grazed (the last leg bypasses cross avoidance). |
| `CLOSE_RADIUS` | 100 | interlock; keep ≥ the planner's 80 px early-exit + grid step. |
| `JITTER_REJECT_SQ` | 60×60 | reject a car-position jump bigger than ~60 px/frame. Set above real max per-frame travel. |
| wall weight | 70 000 | `analyzer/src/path.rs` + `path/src/main.rs`. Re-validate with `measure_clearance.py` per arena; don't exceed ~80 000 at buffer 350. |
| cross weight | 60 000 | same. |

> **Camera-parallax constraint (do not undo):** the hard clearance zones are commented out on purpose. The fixed camera height/angle makes a robot near a wall appear *inside* the wall; a hard exclusion would invalidate the A* start node → no route → the robot stalls. Only ever strengthen avoidance via the soft **weight**, and re-run `measure_clearance.py` to confirm no route caps out.

---

## 6. Residual risks / future follow-ups (not done)

- The penalty weights are validated **on the provided scene only**. Re-run `path/measure_clearance.py` per arena and confirm no route caps out and worst route-end stays < `APPROACH_DISTANCE`.
- Within `APPROACH_DISTANCE` the last leg bypasses **cross** avoidance too; a ball right against the cross could be grazed. Lower `APPROACH_DISTANCE` or gate the bypass to walls-only if observed.
- Pulsed creep is slower than a continuous drive and assumes `stop` brakes within a frame; on a low frame rate it may drift slightly past (the at-or-beyond trigger still latches, so worst case is a slightly off-center pickup, not a runaway).
- Two same-color **touching balls** still merge into one centroid between them (the README's "divide a clump by ball diameter" idea is unimplemented).
- `find_cross()` / `find_map_border()` corner/cross geometry is still visualization-only and feeds nothing into routing.

---

## 7. How to test

```bash
# 1. Offline planner builds + regenerate the route visual:
cargo run --manifest-path path/Cargo.toml      # writes path/map_computed.jpg

# 2. Confirm clearance + no-stall on the real scene (and per new arena):
cd path && python3 measure_clearance.py        # FINAL CHOSEN row: no *CAP*, worstEnd < 200

# 3. On the robot machine (has OpenCV/libclang):
cargo check --manifest-path analyzer/Cargo.toml
# then run analyzer, calibrate SUCK_OFFSET from the blue mouth dot, and watch for
# CREEP / CREEP-BRAKE past the old ~80px stop, SUCK only when the mouth is on the ball.
```

---

## 8. Ground-plane homography — parallax-free pickup *everywhere*

The single-camera rig is oblique, so the relationship between the robot marker (image space) and the ball is perspective-distorted, and the distortion **varies with field position** (worst near the walls). A fixed `SUCK_OFFSET` only absorbs the average. This change rectifies the precision-critical geometry onto the flat arena floor so "mouth on ball" is exact across the whole field.

### What was added
- **New module `analyzer/src/homography.rs`** — a self-contained, **std-only (no OpenCV)** ground-plane homography: a hand-rolled 4-point solve (`getPerspectiveTransform`-equivalent, 8×8 Gaussian elimination with partial pivoting, f64), `apply()` (with a `w≈0` guard), `inverse()`, `from_field_corners()` (rectifies to the corners' own bounding box — **no metric field size needed**), and `correct_marker()` (optional elevated-marker height correction). Includes a `#[cfg(test)]` suite (10 tests).
- **Auto-calibration** — `find_map_border()` already locates the 4 field corners; it now stores `[top_left, top_right, bottom_left, bottom_right]` into `State.last_corners`, but **only when all four are real detections** (rejects any corner still at its frame-corner init sentinel). `main()` caches the solved homography (`ground_h`) next to `last_obst_bounds`, rebuilding only when a corner moves > `CORNER_MOVE_SQ` **and the solve succeeds** — a failed solve keeps the last good H.
- **Surgical integration** — in the close-range block, the ball, robot center, and heading marker are mapped into rectified ground coordinates and the `final_approach` gate, `CLOSE_RADIUS` interlock, mouth projection, `forward_gap`/`lateral`, and `on_ball` are computed there. **A* routing stays in pixel space** (it only needs to deliver the robot into approach range; no routing literals changed, so the `path` crate is not re-mirrored).

### Critical safety properties
- **Flipped/degenerate quads can't corrupt anything.** A mislabeled corner set still *solves* to a finite-but-wrong H that no null-check would catch — so `from_field_corners()` runs a **shoelace signed-area (winding) guard** (a valid image-coord quad walked TL→TR→BR→BL has positive area; a flip is negative → `None`) plus a **25%-of-bbox area guard** (rejects slivers/sentinel-laced sets) *before* building.
- **Graceful fallback.** Whenever the homography is uncalibrated or any `apply()` returns `None`, the block falls back to the **exact** previous integer-pixel expressions (branch on `Option`, no blend) — byte-identical behavior to before this change. The End* state machine and all prior fixes are untouched.

### Robot-marker height correction (default OFF)
The markers sit *above* the floor, so after ground rectification they keep a small residual parallax. `correct_marker` removes it via `corrected = nadir + (apparent − nadir)·k`, with `k = (H_cam − h)/H_cam`. **Default `MARKER_K = 1.0` = identity (disabled)**, so out-of-the-box it's ground-homography-only (a safe improvement needing zero measurements). To enable, measure camera height `H_cam` and marker height `h`, set `MARKER_K`, and set `MARKER_NADIR` to the rectified image of the camera's plumb point.

### New tunables
| Constant | Default | Meaning |
|----------|---------|---------|
| `MARKER_K` | 1.0 (OFF) | Height-correction factor `(H_cam−h)/H_cam`; 1.0 disables it. |
| `MARKER_NADIR` | (0.0, 0.0) | Rectified camera-nadir point; only used when `MARKER_K ≠ 1.0`. |
| `CORNER_MOVE_SQ` | 30² | Per-corner squared-px motion that triggers a homography rebuild. |
| area-ratio guard | 0.25 | Reject a corner quad enclosing < 25% of its bounding box. |

### Verification done here
- The math was validated **independently** by compiling and running both a standalone 22-assertion test **and the module's own 10-test suite** with `rustc` (no OpenCV): all pass — exact corner mapping, inverse round-trip, identity, collinear/`w=0`/degenerate → `None`, marker correction, and the flipped-winding + slim-quad rejections.
- `homography.rs` compiles clean as a library; `main.rs` parses clean and braces balance; A* call sites remain pixel-space.
- ⚠️ The full `analyzer` crate still can't be `cargo build`-ed here (no OpenCV/libclang). **Run `cargo check` in `analyzer/` on the robot machine.** With `MARKER_K=1.0` the only behavioral change vs. before is that the suck decision is computed in rectified coords once the border is detected; if the border isn't visible, behavior is identical to before (pixel fallback).

### Residual notes
- Elevated-marker parallax remains until you calibrate `MARKER_K`/`MARKER_NADIR` (the suck tolerances absorb the small bias meanwhile).
- Bbox-rectified units ≈ pixel scale only when the field roughly fills the frame; on a steeply oblique rig, re-check that a ~50px-from-wall ball still falls inside `APPROACH_DISTANCE` (bump it if needed).
- Corner labeling assumes a roughly axis-aligned border; re-verify if the camera is mounted rotated ≥90°.
