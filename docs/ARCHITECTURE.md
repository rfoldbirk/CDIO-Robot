# CDIO-Robot — Architecture

> DTU CDIO table-tennis-ball-collecting robot. Rust (vision + planning + control) + TypeScript (web control panel) + Python (YOLO experiment).
>
> This document is a synthesis of per-subsystem source reads. File:line references point at the source as read on branch `color-calibration`. Where the code and the original project plan (README) disagree, the **code** is treated as authoritative and the divergence is called out.
>
> **⚠️ Update (branch `color-calibration`):** several issues described below — the AABB ball center, the color-merge in `group_marks`, the buggy jitter clamp, the unbounded close-range SUCK, and the `bounds()` / `get_obstacles()` / `find_cross()` / `find_map_border()` panic hazards — have since been addressed, and the A* wall/cross penalty weights were raised. See [`docs/CHANGES-ball-precision-and-routing.md`](./CHANGES-ball-precision-and-routing.md) for what changed and why.

---

## 1. Overview

### What the robot does

This is the software for a **DTU CDIO course robot** whose assignment is to autonomously **collect table-tennis balls** from an arena and **deliver them into a goal**:

- **Targets:** white ping-pong balls (and, per the assignment, a special **orange** ball — see the orange caveat below).
- **Obstacles to avoid:** the **red walls** bordering the arena, and a **central red cross** obstacle in the middle of the field.
- **Goal:** drive collected balls to a delivery / drop-off position and eject them.
- **Time limit:** a hard **6-minute** countdown forces an end-game delivery sequence regardless of remaining balls (`analyzer/src/main.rs:170`).

The physical robot is a **LEGO EV3 brick** (drive base + collection mechanism) plus a **Raspberry Pi** (suction / vacuum "suck" arm), both driven over TCP from a host PC running the Rust `track-analyzer` binary, which watches the arena through an **overhead webcam**.

> **Orange caveat:** orange is wired in as a *detection* color target, but `State::get_balls` only collects **white** blobs as ball targets (`analyzer/src/main.rs:606-618`). In the current code, orange balls are detected but never actually pursued. See §8.

### The high-level loop

The whole system follows a 4-stage pipeline, named in Danish in the README:

| Stage | Danish | What happens |
|-------|--------|--------------|
| 1. take-image | *Tag billede* | Grab an overhead webcam frame (BGR → RGB). |
| 2. scan | *Scan billede* | Per-pixel color matching → connected-component grouping → classify into car, balls, walls, cross. |
| 3. route | *Lav rute* | Pick the nearest ball (or the delivery position), run a weighted A* grid search around the obstacles. |
| 4. follow | *Følg rute* | Compute heading error vs. the next waypoint, send `left`/`right`/`forward`/`stop`/`pick` to the EV3 and `arm`/`start`/`stop` to the Pi. |

In the **production path**, all four stages run **inline, every frame**, inside `analyzer/src/main.rs`'s infinite loop. The other crates (`track`, `path`) are **earlier offline extractions** of the scan and route stages that operate on JSON/image files instead of a live camera. The `yolo` directory is a separate detection experiment. `control-panel` is a browser UI that spawns these binaries as child processes.

---

## 2. Repository layout

```
/Users/millard/Documents/CDIO-Robot/
├── README.md                  # Game plan + ideas (Danish)
├── .gitignore                 # ignores .vscode/.zed but NOT .idea/
├── analyzer/   crate "track-analyzer" — LIVE camera loop: scan + route + follow + EV3/Pi control
├── path/       crate "path"           — OFFLINE A* route planner + scene visualizer (file-driven)
├── track/      crate "track"          — OFFLINE ball detector (still image → balls.json)
├── yolo/       Python + Rust          — YOLO detection experiment (NO Cargo.toml — not a crate)
└── control-panel/  Node/TS Express UI — operator calibration UI; spawns the Rust binaries
```

### One-line purpose per component

| Component | Language | Purpose |
|-----------|----------|---------|
| `analyzer` (`track-analyzer`) | Rust | The real-time brain: opens the webcam, runs the full color-vision + A* + motor-control loop, talks to EV3 + Pi over TCP. **This is the production system.** |
| `path` | Rust | Standalone offline A* planner that reads `car.json`/`balls.json`/`obstacles.json`, plans a route, and renders it onto `scene.jpg` → `map_computed.jpg`. A file-driven extraction of the analyzer's routing logic. |
| `track` | Rust | Standalone offline ball detector: thresholds a still photo for white/orange blobs, refines centers via 4-ray edge marching, writes `balls.json`. The earlier prototype that `analyzer`'s scan stage superseded. |
| `yolo` | Python + Rust | Experimental neural detector: `main.py` runs an Ultralytics YOLO model → `detections.json`; `src/main.rs` draws the bounding boxes onto an image. Not integrated with the rest. |
| `control-panel` | TypeScript (Express 5) | Browser UI for color calibration (pick hex + precision per color) that spawns the Rust binaries via HTTP routes and displays the annotated result. |

### No Cargo workspace

There is **no workspace `Cargo.toml`** at the repo root. The three Rust crates (`analyzer`, `path`, `track`) are **standalone**, each with its own `Cargo.toml` and `Cargo.lock`, built independently. They even pin **different versions of the `image` crate** (`analyzer`/`track` use `image 0.24`; `path` uses `image 0.25.10`). The `yolo` directory contains a Rust `src/main.rs` but **no `Cargo.toml`** — it is therefore **not buildable as a crate as-is**. So the colloquial "four Rust crates" is really **three crates + one loose Rust+Python directory**.

---

## 3. End-to-end data flow

Components communicate by **writing JSON files in their own directory** and having the next stage read a copy in *its* directory. There is no shared message bus. The `control-panel` orchestrator `spawn()`s each binary with its **cwd = that crate's directory**, so every binary reads/writes JSON relative to its own folder. The handoff of `obstacles.json` / `balls.json` / `car.json` from `analyzer/` to `path/` requires the files to be **copied between directories** — and no code in the inspected orchestrator performs that copy (see Open Questions §9).

> Important reality check: in the **current** `analyzer/src/main.rs`, the only file I/O is reading and writing **`end_pos.json`** (`main.rs:177-179`, `main.rs:326`). The `car.json` / `output.json` / `obstacles.json` / `balls.json` / `scene.jpg` / `map_computed.jpg` artifacts that appear in the repo were produced by an **earlier image-dumping version** of the analyzer (around commits `67874da` "split-up image recognition" / `7f51643` "latest work"). The current camera-loop analyzer no longer emits them. The diagram below shows the **intended file-passing pipeline** the JSON files imply, with that caveat.

```
                         overhead webcam (cv2 device index 0)
                                       │
                                       ▼
   ┌───────────────────────────────────────────────────────────────┐
   │  analyzer  (track-analyzer)  — LIVE per-frame loop             │
   │  scan ─► group ─► classify (car / balls / walls / cross)       │
   │  ─► find_nearest_ball ─► calc_route (A*) ─► next_point          │
   │  ─► heading control ─► TCP commands                            │
   │                                                                │
   │   reads/writes:  end_pos.json   (delivery target)              │
   │   (earlier ver.) car.json, balls.json, obstacles.json,         │
   │                  output.json, scene.jpg                        │
   └───────────────┬───────────────────────────────┬───────────────┘
                   │  TCP 172.20.10.3:5000          │  TCP 172.20.10.11:8000
                   ▼                                ▼
              EV3 brick                        Raspberry Pi
        (drive base + "pick")               (suction "arm/start/stop")


   OFFLINE / FILE-DRIVEN BRANCHES (earlier extractions, not in the live loop):

   still image (analyzer/images/<file>)
        │
        ├─►  track  ──writes──►  track/balls.json
        │        (white/orange blob detection; clustered near-duplicate points)
        │
        └─►  (earlier analyzer)  ──writes──►  output.json / obstacles.json /
                                              car.json / end_pos.json / balls.json
                   │
                   ▼   (JSON copied across dirs — mechanism unknown)
                 path  ──reads──►  path/{obstacles.json, balls.json, car.json}
                       ──reads──►  path/scene.jpg
                       ──writes──►  path/map_computed.jpg   (route visualization)
                       (NB: path/map.jpg exists on disk but is NEVER read by code)


   SEPARATE EXPERIMENT:
   yolo/main.py ──► yolo/detections.json ──► yolo/src/main.rs ──► yolo/output.jpg
```

### File-to-component matrix

| File | Written by | Read by | Notes |
|------|-----------|---------|-------|
| `end_pos.json` | analyzer (`main.rs:326`) | analyzer (`main.rs:177-179`) | Delivery/goal point. The **only** file the current analyzer reads+writes. |
| `car.json` | (earlier analyzer) | `path` (`path/src/main.rs:71-73`) | Single `{x,y}` car position. |
| `balls.json` | `track` (`track/src/main.rs:119`); (earlier analyzer) | `path` (`path/src/main.rs:60-63`); `control-panel` re-displays nothing from it | `track`'s version has near-duplicate clustered points; `path`/`analyzer` versions are a deduplicated 4-point form. |
| `obstacles.json` | (earlier analyzer) | `path` (`path/src/main.rs:61-64`) | `{walls:[…], cross:[…]}` dense pixel clouds (~1 MB+). |
| `output.json` | (earlier analyzer) | **nobody** | Same `{walls,cross}` schema as `obstacles.json`; raw pretty-printed (2.1–2.4 MB). Committed but unread. |
| `scene.jpg` | (input asset) | `path` (`path/src/main.rs:57`) | Background the route overlay is drawn onto. |
| `map.jpg` | (input asset) | **nobody** | Present in `path/` but no code path reads it — stale/leftover. |
| `map_computed.jpg` | `path` (`path/src/main.rs:99`) | (operator eyeballing) | Route + search nodes rendered over `scene.jpg`. |
| `detections.json` | `yolo/main.py` | `yolo/src/main.rs:14` | YOLO bbox contract. |
| `output.jpg` | `yolo/src/main.rs:25` | (operator eyeballing) | Boxes drawn over `image.jpg`. |
| `out.png` | **nobody** | `control-panel` `/latest.png` (`main.ts:66`) | Served by the panel but **never produced** → 404. |

---

## 4. Shared data contract

### Coordinate system

All coordinates are **image pixel coordinates**:

- `x` = horizontal (column), grows rightward.
- `y` = vertical (row), grows **downward**.
- Origin = **top-left** of the frame.
- Units are **pixels** at full camera resolution — there are **no metric/world units** anywhere. Every threshold (the 220-px noise filter, 350/400-px obstacle buffers, 80-px goal tolerance, 10-px jitter clamp) is tuned to a specific camera resolution and overhead mounting that are **not documented in code**. From `path/obstacles.json`, the working frame is roughly **1536×1026**, with walls spanning x≈250–1566, y≈70–1054.

The canonical coordinate type is `Position { x: i32, y: i32 }` (`analyzer/src/main.rs:44-54`). It derives `Hash`/`Eq` so it can be a `HashMap` key (used by flood-fill grouping and by A* node maps), and `Serialize`/`Deserialize` for JSON. `path/` and `track/` redefine structurally identical local `Position` types.

### JSON schemas

| File | Schema | Meaning |
|------|--------|---------|
| `car.json` | `{"x": int, "y": int}` | Single point — robot/car position. |
| `end_pos.json` | `{"x": int, "y": int}` | Single point — delivery/goal position (analyzer-only; falls back to `(0,0)` if missing/unparseable). |
| `balls.json` | `[{"x": int, "y": int}, …]` | Array of ball-candidate centers. `track`'s output has **many near-duplicate** points (no dedup); `path`/`analyzer` carry a small deduplicated set. |
| `obstacles.json` / `output.json` | `{"walls": [{x,y}…], "cross": [{x,y}…]}` | Dense per-pixel point clouds: `walls` = red border pixels, `cross` = the central red cross pixels. Same two-key contract; `output.json` is the raw pretty-printed form, `obstacles.json` the cleaned form. |

Confirmed sizes (illustrative): `path/obstacles.json` → 52,988 wall pts + 5,716 cross pts; `analyzer/output.json` → 43,549 + 4,687; `control-panel/output.json` → 46,223 + 4,787. These multi-MB blobs are **committed to git and churn massively** (100k+ line diffs per commit), bloating history.

The `yolo` branch uses a different contract: `detections.json` = `[{class: string, confidence: float, bbox: [x1,y1,x2,y2]}, …]`.

---

## 5. Component deep-dives

### 5.1 `analyzer` (`track-analyzer`) — the production system

**Purpose.** The central vision + control loop. Opens the overhead webcam, runs the full color-CV pipeline every frame, computes car center/heading, picks a target, routes around obstacles, drives the EV3 + Pi over TCP, and runs an end-game state machine. **This is the one file that matters most** — the others are extractions or experiments of pieces of it.

**Entrypoint / run.** `fn main() -> opencv::Result<()>` at `analyzer/src/main.rs:169`. Crate root declares `pub mod path;` (`main.rs:12`) and `pub mod control;` (`main.rs:13`). Run: `cargo run` (optionally `--release`) inside `analyzer/`, with up to 10 positional CLI color-calibration args. Requires OpenCV installed and a camera at index 0; the EV3/Pi connections are optional (failures are swallowed via `.ok()`).

**Key data structures.**

| Type | Line | Summary |
|------|------|---------|
| `State` | `main.rs:30-36` | Central mutable context: `original_img` (sampled), `img` (annotated working copy, never displayed), `targets: Targets`, `marks: HashMap<Position, Mark>`, `groupings: Vec<Grouping>`. Builder-style methods returning `&mut Self`. |
| `Grouping` | `main.rs:38-42` | One connected component: `marks: HashMap<Position,bool>` (the bool is always `false`/unused — effectively a `HashSet`) + `color: Color`. `volume()` = pixel count; `center()` = **bounding-box midpoint** (`main.rs:576-582`). |
| `Position` | `main.rs:44-54` | `{x,y}` pixel coord; the shared coordinate type. |
| `BetterTarget` | `main.rs:76-81` | A detection target: `color`, `target_hex: Hex`, `precision: u8` (per-channel allowed distance). Five built in `main()`. |
| `Color` | `main.rs:105-114` | Semantic classes: White/Orange/Red/CarCenter/CarDirection/Debug. `.rgb()` returns **display** colors only (misleadingly White→yellow, Red→orange — `main.rs:64-73`). |
| `ProgramState` | `main.rs:116-125` | State machine: Config / Run / EndGoToPos / EndTurnAround / EndOpen / EndShakeItOut / End. |
| `Mark` | `main.rs:127-134` | A matched pixel; `precision` is always stored as 0, and `x`/`y` duplicate the key. |
| **Dead code** | — | `Output` (`22-27`), `Rectangle` (`56-61`), `Target` (`100-103`) are never constructed; `rand` is declared but unused. |

**Key functions & algorithms.**

| Function | Line | Role |
|----------|------|------|
| `main` | `169-541` | 6-min countdown; load `end_pos.json`; connect Pi+EV3; parse 10 CLI args; open camera; infinite loop: read frame → keyboard control → scan pipeline → extract geometry → route → heading control → draw → show. |
| `scan_for_marks` | `831-870` | **Color detector.** Double loop over every pixel × every `BetterTarget`; `check_color` per channel; insert a `Mark` if all 3 channels within precision. O(W·H·5)/frame — the main latency cost. Last-matching-target-wins (HashMap overwrite). |
| `check_color` | `896-907` | Per-channel `abs(pixel − target) ≤ precision`. An axis-aligned **cube** (L∞) test in **RGB** — *not* Euclidean, *not* HSV → lighting-sensitive. |
| `group_marks` | `642-691` | 8-connected **BFS flood fill** over matched pixels into `Grouping`s. **Bug:** does *not* require neighbors to share color — touching blobs of different colors merge, labeled by the seed pixel's color. |
| `filter_groups` | `693-699` | Drops groups with `volume() ≤ 220` px. The main noise filter. |
| `find_borders` | `701-711` | Sorts groups by **descending volume** so index 0 = largest; then calls find_map_border + find_cross. |
| `find_map_border` / `find_cross` | `751-829` / `713-749` | Extremity scans for wall corners / cross arm-tips. **Visualization-only**: they `draw_mark` into the never-displayed `img`, and feed **nothing** into routing in this version. |
| `get_car_center` / `get_car_direction` | `586-604` | First grouping of color CarCenter / CarDirection → bbox center; **returns `(0,0)` if not found** (TODO "Burde nok panikke" = "should probably panic"), silently corrupting heading. |
| `get_balls` | `606-618` | `center()` of every **White** group only (orange excluded). |
| `get_obstacles` | `620-640` | Assumes `groupings[0]` = walls, `groupings[1]` = cross by **size rank**, `.unwrap()` → **panics** if <2 groups survive the filter. |
| `normalize` / `angle_between` | `909-921` | Unit vector (guards zero); signed angle via `atan2(cross, dot)` → sign decides turn direction. |
| `hex_to_rgb` / `map_hex_val` | `136-167` | Manual hex parser; **panics** on malformed/short input. |

**Inputs.** Webcam (cv2 index 0, CAP_ANY); `end_pos.json`; 10 CLI args (hex+precision for red/white/orange/car-center/car-direction); keyboard via `highgui::wait_key` (WASD nudge end_pos ±5px, `1` center, `2` Run+save, `3`–`7` set end-states, `p` arm, ESC quit); TCP ACKs from EV3.

**Outputs.** `end_pos.json` (on `2`); OpenCV window `camera` (annotated overlay); TCP to EV3 `172.20.10.3:5000` (`right`/`left`/`forward`/`backward`/`stop`/`pick`); TCP to Pi `172.20.10.11:8000` (`arm`/`start`/`stop`); stdout timing/debug.

**Hardcoded values.** 6-min countdown (`170`); default colors red `#9D2B5C`/41, white `#FFF8FF`/22, orange `#F4984A`/11, car-center `#198367`/31, car-dir `#D8C456`/31 (`193-226`); volume filter 220 (`695`); jitter clamp 10 px (`413`); heading threshold 7.0° (`472`, comment says originally 15.0); `dist_threshold = 1` (`444`); end-game sleeps 5s/1s/5s, 1s/1s, 3s+1s (`351-366`, `492-496`); EV3/Pi endpoints (`control.rs:5-10`); filename `end_pos.json`.

**Known issues / TODOs.** Orange detected but never pursued; `group_marks` color-mixing bug; `get_obstacles` size-rank `.unwrap()` panic; `(0,0)` fallback for missing car/direction markers; jitter clamp uses **signed** diff so only positive jumps are clamped (`413`); blocking `thread::sleep` (up to 5 s) freezes the camera loop during end-game; `find_map_border`/`find_cross` are dead visualization; two `wait_key` calls/loop can drop keystrokes; misleading `Color::rgb()` display colors; the old JSON/JPG outputs the README expects are gone.

---

### 5.2 `analyzer/src/path.rs` — live routing module

**Purpose.** Weighted A*-style grid search from car → target (ball or `end_pos`), steering away from walls and the cross via **soft cost penalties** (not hard exclusions). Also: next-waypoint selection, route drawing, AABB computation, and greedy nearest-ball selection. Produces geometry only; `main.rs` turns it into motor commands.

**Entrypoint.** Library module (`pub mod path;`, `main.rs:12`); runs inside the per-frame loop (`find_nearest_ball` `main.rs:430`, `calc_route` `main.rs:437`, `draw_route_stream` `main.rs:438`, `next_point` `main.rs:441`).

**Key data structures.** `Node {g_cost, h_cost, parent}` with `f_cost()=g+h` (`path.rs:81-92`); `Bounds {min_x,max_x,min_y,max_y}` (`23-29`); `ObstacleBounds {walls, cross}` (`18-21`); `Obstacles {walls:Vec, cross:Vec}` (`41-45`); `NextInstruction {pos, route_lenth}` (misspelled, `48-51`); `open`/`closed` are `HashMap<Position,Node>` (`187-188`).

**Key functions & algorithms.**

| Function | Line | Role |
|----------|------|------|
| `calc_route` | `182-294` | Weighted A* on an implicit 8-connected grid, **20-px steps**. Pops min `(f_cost, h_cost)`; early-exit within **80 px** of target; **1000-iteration** cap. Soft wall penalty within `WALL_BUFFER=350` (`normalized⁴·50000`); soft cross penalty within `CROSS_BUFFER=400` via `obstacle_penalty`. `HARD_WALL_CLEARANCE`/`HARD_CROSS_CLEARANCE=120` are **commented out** → nothing is truly impassable. |
| `next_point` | `54-78` | Picks the waypoint: short paths → `pos[0]` (near target); long paths → `pos[len-5]` (≈100-px lookahead from car). Match arms for lengths 2–5 are identical (dead branching). Starts from the **global** min-f node, not necessarily the target node. |
| `find_nearest_ball` | `300-318` | Greedy nearest-neighbor by **squared** Euclidean distance. No TSP — one nearest ball at a time, replanned every frame. |
| `dist_to_rect` / `obstacle_penalty` | `320-351` | Point-to-AABB squared distance; quartic repulsion field `normalized⁴·50000`. |
| `bounds` | `32-39` | AABB from a point slice; **`.unwrap()` panics on empty input** (crashes if no walls/cross detected). |
| `get_first_node` | `156-180` | **Dead code** (not called); panics on empty route. |
| `draw_route_stream` | `96-152` | Hardcoded-green route overlay onto the OpenCV frame. |

**Known issues.** Heuristic uses **squared** distance (g also squared, penalties ~50000) → **not admissible**, so this is greedy best-first, not optimal A*. Hard exclusions commented out (biggest correctness risk). Duplicated wall/cross penalty formulas. `open.clone()` every iteration → O(n²). `route_lenth` misspelling. `main.rs` `close_enough` requires `route_lenth < 1`, which combined with the 80-px early-exit may rarely trigger except via the end-state override.

---

### 5.3 `analyzer/src/control.rs` — robot communication layer

**Purpose.** Thin TCP transport. Opens raw sockets to the EV3 and the Pi and sends newline-terminated text commands. No movement logic here — the primitives are string literals sent by `main.rs`.

**Entrypoint.** Library module (`pub mod control;`, `main.rs:13`); `connect_ev3`/`connect_suck`/`send_command` imported at `main.rs:17`.

**Key functions.**

| Function | Line | Role |
|----------|------|------|
| `send_command` | `control.rs:12-34` | `send_command(stream: &mut Option<TcpStream>, cmd: &str, last_command: &mut String)`. De-dups: if `cmd == last_command`, returns early. Writes `"{cmd}\n"`, does one blocking 1024-byte read, prints `EV3: <reply>` (label hardcoded even for the Pi). All socket errors silently swallowed (`let _ =`, `_ => {}`). If `None`, prints `No TCP stream :/`. |
| `connect_ev3` | `36-42` | Blocking connect to `172.20.10.3:5000`. |
| `connect_suck` | `45-57` | Blocking connect to `172.20.10.11:8000`. Has a commented-out forced-failure test block. |

**Command vocabulary.** EV3: `forward`, `backward`, `left`, `right`, `stop`, `pick`. Pi: `arm`, `start`, `stop`. Movement is implied **latched-until-next-command** (consistent with the de-dup design).

**Known issues.** Swallowed socket errors → a dropped connection is invisible, no reconnect. `EV3:` label wrong for Pi replies. Blocking read couples frame rate to device responsiveness. De-dup means a `stop` equal to a prior `stop` is never resent → a missed `stop` could leave motors running. Hardcoded hotspot IPs; no config/CLI override. Unused `Error`/`ErrorKind` imports.

---

### 5.4 `path` crate — standalone offline planner

**Purpose.** File-driven A* + visualizer. Reads `car.json`/`balls.json`/`obstacles.json` + `scene.jpg`, plans a route to the nearest ball, renders search nodes + chosen path onto `map_computed.jpg`. An offline test harness / earlier extraction of the analyzer's routing. **Does not drive the robot** (no follow stage).

**Entrypoint / run.** `fn main()` at `path/src/main.rs:53`. Run `cargo run --release` with **cwd = `path/`** (hardcoded relative paths). No OpenCV — pure `image`-crate rendering (`image 0.25.10`).

**Key functions.** `main` (`53-103`); `calc_route` (`198-310`); `draw_route` (`118-143`); `draw_route_map_debug` (`171-196`); `find_nearest_ball` (`345-363`); `obstacle_penalty` (`385-396`); `get_first_node` (`145-169`, **dead**).

**Algorithm.** Byte-for-byte the same A* as `analyzer/src/path.rs` (20-px grid, 80-px goal, 1000-iter cap, `WALL_BUFFER=350`/`CROSS_BUFFER=400`, `normalized⁴·50000`, squared-distance heuristic and movement cost so diagonals = 800 vs 400). Same dead `HARD_*_CLEARANCE=120` constants. Reads `scene.jpg`, **not** `map.jpg`.

**Hardcoded values.** `RADIUS=40` (**dead, unused**); grid 20; goal 80×80; iter cap 1000; buffers 350/400; penalty 50000/exp 4; draw sizes car 40 / target 20 / balls 10 / nodes 3 / path 7. Color RGB literals at `327-333`.

**Known issues.** Massive duplication with `analyzer/src/path.rs` (two sources of truth that can drift). `open.clone()` per iteration → O(n²). Integer overflow plausible on large maps (squared distances up to ~2.36M accumulated over 1000 iters with 50000 penalties, no checked arithmetic). `bounds`/`find_nearest_ball`/`get_first_node` all panic on empty input. `map.jpg` shipped but unread.

---

### 5.5 `track` crate — standalone ball detector prototype

**Purpose.** Offline single-image experiment: detect white/orange balls in a still photo via coarse color thresholding, refine each candidate's position, filter non-ball blobs via an edge/symmetry test, write `balls.json`. The earlier prototype that `analyzer`'s scan stage superseded.

**Entrypoint / run.** `fn main()` at `track/src/main.rs:51`. Run `cargo run --release -- <image>` (default `images/12.jpg`) with **cwd = `track/`**. No camera, no networking, no routing.

**Key functions & algorithm.**

| Function | Line | Role |
|----------|------|------|
| `main` | `51-122` | Compute global average color; scan in **`INC=10`-px** cells; reject cells whose squared-RGB distance from the global average < **8000** (floor); classify White (`r,g,b>200`) / Orange (`r>190, g>100, b<140`); refine center via 4-ray `edge` marching; filter by roundness/symmetry; `retain`; write `./balls.json`. |
| `edge` | `125-163` | Marches outward in ±direction counting run-length while patch differs from background by > `lim=8022` (note: ≠ the 8000 used in `main`). Used both to re-center and to test roundness. |
| `color_diff` | `165-170` | Squared Euclidean RGB distance from the global average. |

**No real clustering.** Each passing grid cell becomes its **own independent Mark**; the re-centering nudges them toward the ball center but **never merges** them → one ball yields several near-identical points. The committed `track/balls.json` has 54 entries but only 46 unique points (e.g. four at exactly `625,495`). This is the **near-duplicate-points** issue, fixed in `analyzer` by flood-fill grouping + per-group center.

**Known issues.** No dedup/clustering (the duplicate-points cause); `out.jpg` is a **stale artifact** (current code never writes it); `8000` vs `8022` threshold inconsistency; an average-area divide bug at image edges in `get_avg_color_in_pixel_field_wh` (`183-209`); `state.img` cloned but unused; RGB box thresholds brittle to lighting; profane Danish `.expect()` panic messages.

---

### 5.6 `yolo` directory — neural-detection experiment

**Purpose.** A "yolo start" spike replacing color-distance detection with an Ultralytics YOLO model. `main.py` runs the model on a still image → `detections.json`; `src/main.rs` redraws the boxes → `output.jpg`. Detect-and-visualize only; not integrated with routing/control.

**Entrypoint / run.** Python: `python yolo/main.py` (needs `ultralytics`, `cv2`, weights `yolo26m.pt`). Rust: **not buildable — no `Cargo.toml`**.

**Key functions.** `main.py` (`1-31`): loads `YOLO('yolo26m.pt')`, opens the webcam **twice but never reads a frame** (dead), runs detection on hardcoded `image.jpg`, dumps every box (no confidence/class filter) to `detections.json`. Rust `main` (`13-28`) + `draw_detection` (`30-46`).

**Known issues.** No `Cargo.toml` → not a crate. **Border-draw bug** (`src/main.rs:39`): `x == x || y == y` is a tautology → fills the whole box. Unclamped `put_pixel` → runtime panic on edge boxes. Pointless single-arm `match` ignores class. Ignored `save` Result. Webcam opened twice, unused. No confidence/class filter (balls not distinguished from anything). `yolo26m.pt` is not a known published Ultralytics weight (likely a custom/local file or typo for `yolo11m.pt`). No bridge into `Position`/`balls.json` or `control.rs`.

---

### 5.7 `control-panel` — web UI / orchestrator

**Purpose.** Browser-based operator UI for color calibration. Lets the operator pick hex colors + per-color precision and trigger the Rust binaries on an image, then shows the annotated result. Express server is a thin shim that `spawn`s the compiled binaries with calibration as CLI args.

**Entrypoint / run.** `control-panel/main.ts` (`"type":"module"`); `app.listen(3000)` at `main.ts:69`. **No `build`/`start` npm script** (only a failing placeholder `test`); run manually via a TS runner (e.g. `npx tsx main.ts`) with **cwd = `control-panel/`**. Browse `http://localhost:3000/`.

**Routes.**

| Route | Line | Behavior |
|-------|------|----------|
| `/`, `/style.css` | `8-14` | Serve `public/index.html` / `public/style.css` from `process.cwd()`. |
| `/balls` | `17-24` | Spawn `../track/target/release/track <../analyzer/images/filename>`. |
| `/obstacles` | `39-52` | Spawn `../analyzer/target/release/track-analyzer` with `[image, white, orange, red, wp, op, rp]`. |
| `path` (no leading `/`) | `55-61` | **Broken** — route never matches; never sends a response. |
| `/latest.png` | `65-67` | Serves `./out.png` — **does not exist** → 404. |
| `react_to_process` | `75-93` | Waits for stdout token `executed in: `; 600ms failsafe → `{ok:false}`. |
| `start`/`stop` | `26-36` | **Dead** — not wired to any route; would `spawn('../yolo')` (a directory, not executable); `stop()` uses `clearTimeout` on intervals (wrong). |

**Frontend (`public/index.html`).** Color pickers + sliders + filename; calibration persisted to `localStorage` (`tmp_saved_values`/`ic_saved_values`); `go()`/`_go()` debounce (400 ms) then sequentially fetch `/obstacles` → `/balls` → `/latest.png`.

**Critical issues.**
- **Arg-order mismatch:** the panel passes `[image, white, orange, red, wp, op, rp]` but the analyzer reads `[red, rp, white, wp, orange, op, …]` (`analyzer/src/main.rs:193-212`). Colors and precisions are **scrambled**.
- **Stdout-contract mismatch:** `react_to_process` waits for `executed in: ` but only `track` prints that — the analyzer is a camera loop printing `frame took {}ms`, so `/obstacles` always hits the failsafe `{ok:false}`.
- **Analyzer is a camera loop, not a one-shot:** it opens `VideoCapture(0)` and ignores the image-path arg the panel is built around. The whole image-file workflow predates the camera rewrite.
- `/latest.png` 404s; `path` route bug; `start`/`stop` dead; filename default `14.png` vs real `14.jpg`; **path traversal** possible (filename concatenated unvalidated into the spawn arg); 2.4 MB `obstacles.json`/`output.json` committed but never read by the server.

---

## 6. The vision pipeline explained

The production vision pipeline lives entirely in `analyzer/src/main.rs` (`scan_for_marks` → `group_marks` → `filter_groups` → `find_borders`, plus geometry extractors). `track` is an earlier, less robust take on just the ball-detection part.

### 6.1 Color detection (per-pixel matching)

`scan_for_marks` (`main.rs:831-870`) loops over **every pixel** and **every** of the 5 `BetterTarget`s (red, white, orange, car-center, car-direction). For each, `check_color` (`main.rs:896-907`) tests **each RGB channel independently**: `abs(pixel_channel − target_channel) ≤ precision`. A pixel matches a target only if **all three** channels pass.

Geometrically this is an **axis-aligned cube** (L∞ box) around the target color in **RGB space** — *not* a Euclidean ball, and *not* HSV. Consequence: it is **sensitive to brightness/lighting changes**, which is exactly why the README and git history are full of "less confusion from light" tuning and why the `color-calibration` branch exists. The hex+precision CLI args (and the control-panel sliders) let the operator widen/narrow each cube per color. A pixel that matches two targets is overwritten by the **later** target in the list (HashMap insert, last-wins).

`track` does a cruder variant: it averages 10×10 cells, rejects "floor-colored" cells via a global-average background-distance threshold, then applies fixed RGB box rules (`r,g,b>200` = white; `r>190,g>100,b<140` = orange).

### 6.2 Pixel clustering (connected components)

`group_marks` (`main.rs:642-691`) performs an **8-connected BFS flood fill** over the set of matched pixels, using a `taken` visited set. Each `Grouping` inherits the color of its **seed pixel**. **Critical limitation:** the BFS does **not** check that a neighbor shares the seed's color — any adjacent matched pixel is merged. So touching blobs of *different* colors (e.g. a white ball against the red wall) merge into one mislabeled group.

`filter_groups` (`main.rs:693-699`) then drops every group with `volume() ≤ 220` px — the primary noise filter.

`track` has **no clustering at all** — each passing grid cell is its own point, which is why its `balls.json` is full of near-duplicates.

### 6.3 Ball-center finding

`Grouping::center` (`main.rs:576-582`) returns the **bounding-box midpoint** `((min_x+max_x)/2, (min_y+max_y)/2)` — the AABB center, **not** the pixel centroid (mean). This is fast but sensitive to outlier pixels and asymmetric blobs. Each surviving **white** group yields one ball position via `get_balls` (`main.rs:606-618`); the single CarCenter and CarDirection groups yield the robot center and heading point.

The README proposed a more sophisticated "measure distance from center to four points" idea and "divide a clump by ping-pong-ball height; pick the modal diameter as true size." `track`'s `edge` 4-ray marching is the partial realization of the 4-points idea; the modal-diameter clump-splitting was never implemented.

### 6.4 Wall & cross detection

`find_borders` (`main.rs:701-711`) sorts groups by **descending volume**, making index 0 = largest blob. `get_obstacles` (`main.rs:620-640`) then assumes **`groupings[0]` = walls** and **`groupings[1]` = cross**, purely by size rank, and returns their raw pixel point clouds. This is fragile: it `.unwrap()`s (panics if <2 groups survive the filter) and silently mislabels if a ball blob ever outsizes the cross.

`find_map_border` (`main.rs:751-829`) and `find_cross` (`main.rs:713-749`) compute corner / cross-arm-tip geometry by extremity scanning (partitioning the image into quarters, snapping the cross's opposite extremes to its mid-axis). **However**, in this version they only `draw_mark` into the never-displayed working `img` — their outputs feed **nothing** into routing. They are effectively dead visualization. Routing consumes only the AABBs that `path::bounds` derives from the raw wall/cross point clouds.

---

## 7. The two pathfinding implementations

There are **two near-identical copies** of the same weighted A* planner:

| Aspect | `analyzer/src/path.rs` (live) | `path` crate (`path/src/main.rs`, offline) |
|--------|-------------------------------|---------------------------------------------|
| Form | Library module of `track-analyzer` | Standalone binary |
| Input | Live per-frame geometry from the scan stage | `car.json` / `balls.json` / `obstacles.json` files |
| Output | `NextInstruction` waypoint → `main.rs` motor commands | Renders `map_computed.jpg`; **no follow stage** |
| Rendering | OpenCV (`draw_route_stream`) | `image` crate (`draw_route` / `draw_route_map_debug`) |
| `image` version | 0.24 | 0.25.10 |
| `Obstacles` derive | `Serialize` (it *produces* obstacles) | `Deserialize` (it *consumes* them) |
| `next_point` lookahead | **Yes** (`pos[len-5]`) | No — has dead `get_first_node` instead |

The **core algorithm is byte-for-byte identical**: implicit 20-px 8-connected grid, `f=g+h` with `h` = **squared** Euclidean distance and `g` accumulating squared step costs (diagonals 800 vs orthogonals 400) plus quartic obstacle penalties; early-exit within 80 px; 1000-iteration cap; `WALL_BUFFER=350`, `CROSS_BUFFER=400`, penalty `normalized⁴·50000`; `HARD_WALL_CLEARANCE`/`HARD_CROSS_CLEARANCE=120` **commented out** in both. `calc_route`, `find_nearest_ball`, `dist_to_target`, `dist_to_rect`, `obstacle_penalty`, `Node`, `Bounds`, `ObstacleBounds`, `bounds`, `get_first_node` are duplicated.

**Evolution & duplication.** Evidence (the `Serialize` vs `Deserialize` direction on `Obstacles`, no OpenCV/`next_point`, `image 0.25` vs `0.24`) indicates the **`path` crate is the older/standalone extraction**, and `analyzer/src/path.rs` is the productionized successor that added the live-loop integration (`next_point` lookahead, OpenCV drawing). They are now **two sources of truth that can drift**. The `analyzer` version is canonical for the running robot; the `path` crate is an offline replay/debug tool.

**Shared algorithmic caveat (both).** Because the heuristic uses **squared** distance, A* is **not admissible** — this is greedy best-first, not optimal A*. And because the **hard exclusion zones are commented out**, the planner can route straight through a wall or the cross if the goal pull outweighs the soft penalty. These are the two biggest correctness risks in routing.

---

## 8. Known issues, tech debt & duplication

### Structure / build
- **No Cargo workspace** — three independent crates with separate lockfiles and **divergent `image` versions** (0.24 vs 0.25.10).
- **`yolo/` has no `Cargo.toml`** — the Rust half is not buildable as-is; you'd need to add a manifest (`image`, `serde` derive, `serde_json`).
- **No `build`/`start` script** in `control-panel/package.json` (only a failing `test`); `"main":"index.js"` references a file that is never produced.
- Multi-MB JSON blobs (`obstacles.json`/`output.json`, 2–2.4 MB) are **committed and churn massively** (100k+ line diffs), bloating git history.
- `.idea/` is present but **not git-ignored**.

### Massive duplication / multiple overlapping implementations
- The **A* planner is duplicated** between `analyzer/src/path.rs` and the `path` crate (see §7).
- **Ball detection exists in two places**: `track` (offline, no clustering) and `analyzer`'s scan stage (live, flood-fill). `track` is effectively superseded/frozen.
- **`Position` and `Color` are redefined** in each crate rather than shared.
- The wall-penalty formula is **inlined** in `calc_route` instead of reusing `obstacle_penalty` (and uses linear vs squared distance conventions inconsistently).

### Correctness bugs
- **Hard obstacle clearances commented out** in both planners → routes can cut through walls/cross (§7). Biggest routing risk.
- **Non-admissible squared-distance heuristic** → greedy, not optimal.
- `group_marks` **merges different-colored adjacent blobs** (§6.2).
- `get_obstacles` **`.unwrap()` panics** if <2 groups survive; mislabels by size rank.
- `get_car_center`/`get_car_direction` return **`(0,0)`** on missing markers → corrupt heading (TODO to panic).
- Jitter clamp uses **signed** diff (only clamps positive jumps) and then overwrites `last_car_pos` with the clamped value, so a real fast move sticks (`main.rs:413`).
- **control-panel ↔ analyzer arg-order mismatch** scrambles colors/precisions; **stdout-contract mismatch** makes `/obstacles` always fail; `/latest.png` 404s; the `path` route is missing a leading `/`.
- `yolo/src/main.rs` border-draw tautology fills boxes; unclamped `put_pixel` panics on edge boxes.

### Robustness / behavior
- **Blocking `thread::sleep`** (up to 5 s) and **blocking socket reads** sit inside the camera loop → freeze vision during end-game and on slow device replies.
- **All socket errors swallowed** (`let _ =`, `_ => {}`); no reconnect; the robot can run "blind" silently.
- **Command de-dup** can drop a re-sent `stop` → motors could stay latched.
- `bounds`/`find_nearest_ball`/`get_first_node` **panic on empty input**.
- `hex_to_rgb`/CLI `.expect("Naah")` **panic** on malformed input at startup.

### Data quality
- **Near-duplicate points in `balls.json`** from `track` (no clustering/dedup) — one physical ball → several points (§5.5).
- **Orange balls detected but never pursued** (`get_balls` is white-only).
- **Stale artifacts:** `out.jpg` (track), `map.jpg` (path, unread), `out.png` (panel, never produced), `output.json` (committed, unread).

### Dead code
- `Output`/`Rectangle`/`Target` structs; `find_map_border`/`find_cross` (visualization-only); `get_first_node`; `RADIUS` const; `Color::Obstacle`; `rand` dependency; `start`/`stop` in the panel; unused imports across files.

---

## 9. Open questions for the author

1. **Camera vs. file workflow:** Is the system meant to run against **live camera** (current analyzer) or **image files** (what the control-panel + `track` + `path` JSON pipeline assume)? The two have diverged.
2. **Canonical analyzer CLI signature:** The README, the analyzer's actual arg order (`red, rp, white, wp, orange, op, car-center…`), and `control-panel`'s order (`image, white, orange, red, wp, op, rp`) **all disagree**. Which is authoritative, and should they be reconciled?
3. **JSON handoff mechanism:** Each binary reads/writes JSON in its own cwd. How do `obstacles.json`/`balls.json`/`car.json` get **copied from `analyzer/` to `path/`**? Is there an external script, or is it manual? Are the committed JSONs live dumps or hand-authored fixtures?
4. **Where should the annotated result image be written** so `/latest.png` finds it (`control-panel/out.png`)? The analyzer writes images into `analyzer/`, not the panel dir — is a copy/symlink step missing?
5. **Hard clearance zones:** Were `HARD_WALL_CLEARANCE`/`HARD_CROSS_CLEARANCE` commented out **intentionally for tuning**, or is that an accidental regression letting the planner cross obstacles?
6. **Squared-distance heuristic:** deliberate goal-biasing choice, or an oversight that should be true Euclidean distance? It materially changes path optimality and the diagonal bias.
7. **Orange-ball collection:** intended scope cut or a bug? Orange is calibrated and detected but `get_balls` ignores it.
8. **`find_map_border`/`find_cross`:** intentionally vestigial debug, or were their corner/cross outputs supposed to feed routing (currently they only draw into an unused image)?
9. **YOLO direction:** Is YOLO meant to **replace** the color detector or only **complement** it (e.g. find the robot/cross while color finds balls)? And is `yolo26m.pt` a custom-trained model or a typo for a stock weight?
10. **EV3/Pi protocol:** What exactly do `pick`/`arm`/`start`/`stop` do physically, are drive commands latched-until-next-command, and what ACK strings come back? (Defined on the robot side, not in this repo.)
11. **Arena/camera spec:** What are the real resolution, mounting, and orientation? Every threshold (220-px filter, 350/400-px buffers, 80-px goal, 10-px jitter) is tuned to an undocumented image size.
12. **Timeout behavior:** Should the 6-minute countdown match the competition limit, and is abandoning remaining balls at timeout (forcing `EndGoToPos`) the desired behavior?
13. **`track`/`path`/`yolo` status:** Are these frozen prototypes, or still maintained? If frozen, should the duplicated A* and ball-detection be deleted in favor of `analyzer`?

---

## 10. How to build & run each component

There is no root build script; each component is built/run independently. Production paths use **release** builds (the control panel calls `target/release/...`).

### analyzer (`track-analyzer`) — the live robot
```bash
cd /Users/millard/Documents/CDIO-Robot/analyzer
cargo build --release            # requires system OpenCV (opencv 0.98.2 bindings)
cargo run --release -- \
  "#9D2B5C" 41 "#FFF8FF" 22 "#F4984A" 11 "#198367" 31 "#D8C456" 31
# args: red_hex rp white_hex wp orange_hex op carcenter_hex cp cardir_hex dp
# Needs: a camera at index 0; EV3 reachable at 172.20.10.3:5000 and Pi at
# 172.20.10.11:8000 (both optional — failures are tolerated, robot runs "blind").
# Keyboard in the OpenCV window: WASD move end_pos, 1 center, 2 Run+save,
# 3-7 set end-states, p arm, ESC quit.
```

### path — offline route planner / visualizer
```bash
cd /Users/millard/Documents/CDIO-Robot/path
cargo build --release
cargo run --release               # MUST be run from path/ (hardcoded relative paths)
# Reads scene.jpg + car.json + balls.json + obstacles.json; writes map_computed.jpg.
```

### track — offline ball detector
```bash
cd /Users/millard/Documents/CDIO-Robot/track
cargo build --release
cargo run --release -- images/12.jpg   # arg optional; default images/12.jpg
# MUST be run from track/. Writes ./balls.json. (Does NOT write out.jpg — stale.)
```

### yolo — detection experiment
```bash
# Python detector:
cd /Users/millard/Documents/CDIO-Robot/yolo
python main.py                    # needs: ultralytics, opencv-python, weights yolo26m.pt
                                  # runs on hardcoded image.jpg → detections.json

# Rust visualizer: NOT buildable as-is — no Cargo.toml. To build it you must add a
# manifest (e.g. copy path/Cargo.toml's deps: image 0.25.10, serde 1.0 + derive,
# serde_json 1.0, edition 2024), then:  cargo run --release  → output.jpg
```

### control-panel — web UI
```bash
cd /Users/millard/Documents/CDIO-Robot/control-panel
npm install                       # express ^5.2.1
npx tsx main.ts                   # no build/start script exists; run the TS directly
                                  # (or ts-node). MUST be launched from control-panel/.
# Pre-build the spawned binaries first (cargo build --release in ../track, ../analyzer,
# ../path). Then open http://localhost:3000/.
# NOTE: /obstacles currently fails (arg-order + stdout-contract mismatch with the
# camera-loop analyzer), and /latest.png 404s — see §8.
```

---

*End of architecture document.*
