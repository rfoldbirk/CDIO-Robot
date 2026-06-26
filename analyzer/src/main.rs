use image::{Rgb, RgbImage};
use opencv::core::Vector;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{HashMap, VecDeque};
use std::env;
use std::error::Error;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub mod path;
pub mod control;
pub mod homography;

use opencv::{core, highgui, imgproc, prelude::*, types, videoio};

use crate::control::{connect_ev3, connect_suck, send_command};
use crate::path::{Node, ObstacleBounds, Obstacles, bounds, calc_route, draw_route_stream, find_nearest_ball, get_first_node, next_point};

const MAX: i32 = 100000;

#[derive(Serialize)]
struct Output {
    walls: Vec<Position>,
    cross: Vec<Position>,
    // balls: Vec<Ba>
}

// State
struct State {
    original_img: RgbImage,
    img: RgbImage,
    targets: Targets,
    marks: HashMap<Position, Mark>,
    groupings: Vec<Grouping>,
    // Last 4 field-border corners [top_left, top_right, bottom_left,
    // bottom_right] in image pixels, stored by find_map_border ONLY when all
    // four are real detections (none still at its init sentinel). None until
    // the border has been seen cleanly once. Used to build/refresh the cached
    // ground homography in main(); never read by routing.
    last_corners: Option<[Position; 4]>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct Grouping {
    marks: HashMap<Position, bool>,
    color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

struct Rectangle {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Color {
    fn rgb(&self) -> Rgb<u8> {
        match self {
            Color::White => Rgb([247, 246, 5]),
            Color::Orange => Rgb([255, 165, 80]),
            Color::Red => Rgb([255, 125, 20]),
            Color::CarCenter => Rgb([100, 255, 100]),
            Color::CarDirection => Rgb([100, 255, 150]),
            Color::Debug => Rgb([222, 193, 132]),
        }
    }
}

#[derive(Debug, Clone)]
struct BetterTarget {
    color: Color,
    target_hex: Hex,
    precision: u8,
}

#[derive(Debug, Clone)]
struct Hex {
    hex: String,
    rgb: Rgb<u8>,
}

impl Hex {
    fn new(hex: String) -> Hex {
        let rgb = hex_to_rgb(&hex);

        Self { hex, rgb }
    }
}

#[derive(Debug, Clone)]
struct Targets(Vec<BetterTarget>);

struct Target {
    red: Rgb<u8>,
    red_precision: u8,
}

#[derive(Default, Hash, PartialEq, Eq, Debug, Clone, Copy)]
enum Color {
    #[default]
    White,
    Orange,
    Red,
    CarCenter,
    CarDirection,
    Debug,
}

#[derive(PartialEq, Eq, Debug)]
enum ProgramState {
    Config,
    Run,
    EndGoToPos,
    EndTurnAround,
    EndOpen,
    EndShakeItOut,
    End
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
struct Mark {
    /// distance from target color
    precision: i32,
    color: Color,
    x: i32,
    y: i32,
}

fn map_hex_val(character: Option<char>) -> u8 {
    match character.unwrap() {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        'A' => 10,
        'B' => 11,
        'C' => 12,
        'D' => 13,
        'E' => 14,
        'F' => 15,
        _ => panic!("Invalid Hex Value!"),
    }
}

fn hex_to_rgb(hex: &str) -> Rgb<u8> {
    let hex = hex.split("#").last().unwrap().to_uppercase();
    let mut chars = hex.chars();

    let r = map_hex_val(chars.nth(0)) * 16 + map_hex_val(chars.nth(0));
    let g = map_hex_val(chars.nth(0)) * 16 + map_hex_val(chars.nth(0));
    let b = map_hex_val(chars.nth(0)) * 16 + map_hex_val(chars.nth(0));

    Rgb([r, g, b])
}

fn main() -> opencv::Result<()> {
    let countdown = std::time::Duration::from_secs(60 * 6);
    let start = std::time::Instant::now();

    let mut program_state = ProgramState::Config;

    // in the center
    let mut end_pos = Position::new(0, 0);
    if let Ok(file) = std::fs::read_to_string("end_pos.json") {
        end_pos = serde_json::from_str(&file).unwrap_or(end_pos);
    }

    let mut last_car_pos = Position::new(0, 0);

    let mut pi: Option<TcpStream> = connect_suck().ok();
    let mut ev3: Option<TcpStream> = connect_ev3().ok();
    let mut pi_last_command = String::new();
    let mut last_command = String::new();


    send_command(&mut pi, "arm", &mut pi_last_command);
    send_command(&mut ev3, "close", &mut pi_last_command);

    // load farve værdier fra kommandolinjen
    // precision values (unøjagtigheder)
    let red_hex: String = env::args().nth(1).unwrap_or("#9D2B5C".to_string());
    let rp: u8 = env::args()
        .nth(2)
        .unwrap_or("41".to_string())
        .parse()
        .expect("Naah");

    let white_hex: String = env::args().nth(3).unwrap_or("#FFF8FF".to_string());
    let wp: u8 = env::args()
        .nth(4)
        .unwrap_or("42".to_string())
        .parse()
        .expect("Naah");

    let orange_hex: String = env::args().nth(5).unwrap_or("#F4984A".to_string());
    let op: u8 = env::args()
        .nth(6)
        .unwrap_or("11".to_string())
        .parse()
        .expect("Naah");

    let car_center: String = env::args().nth(7).unwrap_or("#1C937A".to_string());
    let car_center_precision: u8 = env::args()
        .nth(8)
        .unwrap_or("41".to_string())
        .parse()
        .expect("Naah");

    let car_direction: String = env::args().nth(9).unwrap_or("#DECA5B".to_string());
    let car_direction_precision: u8 = env::args()
        .nth(10)
        .unwrap_or("41".to_string())
        .parse()
        .expect("Naah");

    // Open camera
    let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?;

    if !cam.is_opened()? {
        panic!("Could not open camera");
    }

    let mut frame = Mat::default();
    cam.read(&mut frame)?;
    let mut rgb = Mat::default();
    imgproc::cvt_color(
        &frame,
        &mut rgb,
        imgproc::COLOR_BGR2RGB,
        0,
        core::AlgorithmHint::ALGO_HINT_ACCURATE,
    )?;

    let bytes = rgb.data_bytes().unwrap();
    let img = RgbImage::from_raw(rgb.cols() as u32, rgb.rows() as u32, bytes.to_vec()).unwrap();

    let mut state = State {
        original_img: img.clone(),
        img,
        targets: Targets(vec![
            BetterTarget {
                color: Color::Red,
                target_hex: Hex::new(red_hex),
                precision: rp,
            },
            BetterTarget {
                color: Color::White,
                target_hex: Hex::new(white_hex),
                precision: wp,
            },
            BetterTarget {
                color: Color::Orange,
                target_hex: Hex::new(orange_hex),
                precision: op,
            },
            BetterTarget {
                color: Color::CarCenter,
                target_hex: Hex::new(car_center),
                precision: car_center_precision,
            },
            BetterTarget {
                color: Color::CarDirection,
                target_hex: Hex::new(car_direction),
                precision: car_direction_precision,
            },
        ]),
        marks: std::collections::HashMap::new(),
        groupings: Vec::new(),
        last_corners: None,
    };

    let mut target = None;

    // Last successfully-detected obstacle bounds. Walls and the cross are STATIC
    // in the arena, so we cache them and reuse on frames where detection drops a
    // blob, instead of panicking. Stays None until the border is seen once.
    let mut last_obst_bounds: Option<ObstacleBounds> = None;

    // Cached GROUND-PLANE HOMOGRAPHY and the corner set it was built from.
    // Like last_obst_bounds, the field border is STATIC, so we solve the
    // homography once the corners are seen cleanly and reuse it. Recompute only
    // when not yet set OR a corner has moved more than CORNER_MOVE_SQ (squared
    // px) since the cached build AND the rebuild SUCCEEDS — a failed rebuild
    // (degenerate/flipped/slim/singular) keeps the last good H rather than
    // caching a bad one or stalling, so the cache can never latch a wrong H.
    let mut ground_h: Option<homography::Homography> = None;
    let mut h_corners: Option<[Position; 4]> = None;
    // Per-corner move threshold (squared px) to trigger a homography rebuild.
    // Above per-frame border jitter, below a real re-aim. Tune if the border
    // detection is noisy. ~30px => 900.
    const CORNER_MOVE_SQ: i32 = 30 * 30;


    loop {
        println!(" ---- {program_state:?} ----");
        let elapsed_so_far = start.elapsed();
        if elapsed_so_far >= countdown {
            program_state = ProgramState::EndGoToPos;
        }


        let frame_start = std::time::Instant::now();
        cam.read(&mut frame)?;

        if frame.empty() {
            continue;
        }

        let key = highgui::wait_key(1)?;

        // w = 119
        // a = 97
        // d = 100
        // s = 115

        match key {
            119 => end_pos.y -= 5,
            97 => end_pos.x -= 5,
            100 => end_pos.x += 5,
            115 => end_pos.y += 5,
            112 => send_command(&mut pi, "arm", &mut pi_last_command),
            49 => {
                // find approximate center
                end_pos.x = frame.cols() as i32 / 2;
                end_pos.y = frame.rows() as i32 / 2;
            },
            50 => {
                program_state = ProgramState::Run;

                // save the position!
                let _ = std::fs::write("end_pos.json", serde_json::to_string(&end_pos).unwrap());
            }, // Press 2
            51 => program_state = ProgramState::EndGoToPos, // press 3
            52 => program_state = ProgramState::EndTurnAround, // press 4
            53 => program_state = ProgramState::EndOpen, // press 5
            54 => program_state = ProgramState::EndShakeItOut, // press 6
            55 => program_state = ProgramState::End, // press 7
            _ => {}
        }


        // draw end_position :)
        let color = Color::Debug.rgb();
        draw(&mut frame, end_pos, (color[0] as f64, color[1] as f64, color[2] as f64))?;



        match program_state {
            ProgramState::EndGoToPos => {
                target = Some(end_pos);
            },
            ProgramState::EndOpen => {
                // TODO: Send open signal
                // START SUCKING
                send_command(&mut pi, "start", &mut pi_last_command);
                std::thread::sleep(Duration::from_secs(5));
                send_command(&mut ev3, "pick", &mut last_command);
                std::thread::sleep(Duration::from_secs(1));
                send_command(&mut ev3, "stop", &mut last_command);
                // STOP_SUCK();
                send_command(&mut pi, "stop", &mut pi_last_command);
                std::thread::sleep(Duration::from_secs(5));

                program_state = ProgramState::EndShakeItOut;
            },
            ProgramState::EndShakeItOut => {
                send_command(&mut ev3, "forward", &mut last_command);
                std::thread::sleep(Duration::from_secs(1));
                send_command(&mut ev3, "backward", &mut last_command);
                std::thread::sleep(Duration::from_secs(1));
                send_command(&mut ev3, "stop", &mut last_command);

                program_state = ProgramState::End;
            },
            ProgramState::End => {
                println!("\n Done :)");
                return Ok(());
            }
            _ => {}
        }


        let mut rgb = Mat::default();
        imgproc::cvt_color(
            &frame,
            &mut rgb,
            imgproc::COLOR_BGR2RGB,
            0,
            core::AlgorithmHint::ALGO_HINT_ACCURATE,
        )?;


        let bytes = rgb.data_bytes().unwrap();
        let img = RgbImage::from_raw(rgb.cols() as u32, rgb.rows() as u32, bytes.to_vec()).unwrap();

        state.img = img.clone();
        state.original_img = img;

        state.marks.clear();
        state.groupings.clear();


        state
            .scan_for_marks() // finder alle pixels der har samme farve som de valgte farver - eller i det mindste er tæt nok på
            .group_marks() // grupperer pixels som ligger op ad hinanden
            .filter_groups()
            .find_borders();


        // Get car from picture!
        let mut car_center = state.get_car_center();

        // Magnitude-based jitter clamp.
        // OLD BUG: the signed `> 10` test only caught jumps toward +x/+y
        // (never -x/-y), used a tiny 10px threshold the car EXCEEDS while
        // driving, and on a real move both rejected it AND wrote the stale
        // value back into last_car_pos -> the car position froze permanently
        // once it moved fast, poisoning the heading vector and the route.
        // NEW: reject only true outliers by squared distance, and update
        // last_car_pos ONLY with the ACCEPTED value so a real drive is never
        // frozen. JITTER_REJECT_SQ must exceed the largest legitimate
        // per-frame travel (measure on the robot).
        const JITTER_REJECT_SQ: i32 = 60 * 60; // ~60px/frame; tune above real max travel
        if last_car_pos.x == 0 && last_car_pos.y == 0 {
            last_car_pos = car_center;
        } else {
            let d2 = (car_center.x - last_car_pos.x).pow(2)
                + (car_center.y - last_car_pos.y).pow(2);
            if d2 > JITTER_REJECT_SQ {
                car_center = last_car_pos; // reject this frame's outlier
            } else {
                last_car_pos = car_center; // accept and remember
            }
        }

        let head = state.get_car_direction();
        let balls = state.get_balls();
        // Obstacle bounds (walls + cross AABBs). Walls and the cross are STATIC,
        // so cache the last good bounds and reuse them on frames where detection
        // momentarily drops a blob. Previously get_obstacles() and bounds()
        // panicked (.unwrap()) whenever <2 groups survived or a point cloud was
        // empty, crashing the whole loop ("robot doesn't exist").
        if let Some(obstacles) = state.get_obstacles() {
            if let (Some(walls), Some(cross)) =
                (bounds(&obstacles.walls), bounds(&obstacles.cross))
            {
                last_obst_bounds = Some(ObstacleBounds { walls, cross });
            }
        }

        // Refresh the cached ground homography from the latest validated border
        // corners. Rebuild only when unset or a corner moved past the threshold,
        // and ONLY commit a rebuild that actually succeeds: from_field_corners
        // returns None on a degenerate/flipped/slim/singular quad, in which case
        // we keep the last good H (or stay None and fall back to pixel space).
        if let Some(corners) = state.last_corners {
            let need_rebuild = match h_corners {
                None => true,
                Some(prev) => {
                    let mut moved = false;
                    for i in 0..4 {
                        let dx = corners[i].x - prev[i].x;
                        let dy = corners[i].y - prev[i].y;
                        if dx * dx + dy * dy > CORNER_MOVE_SQ {
                            moved = true;
                        }
                    }
                    moved
                }
            };
            if ground_h.is_none() || need_rebuild {
                let pts = [
                    (corners[0].x as f64, corners[0].y as f64),
                    (corners[1].x as f64, corners[1].y as f64),
                    (corners[2].x as f64, corners[2].y as f64),
                    (corners[3].x as f64, corners[3].y as f64),
                ];
                if let Some(h) = homography::from_field_corners(pts) {
                    ground_h = Some(h);
                    h_corners = Some(corners);
                }
                // else: keep last good ground_h / h_corners unchanged.
            }
        }

        if target.is_none() {
            target = find_nearest_ball(&car_center, &balls);

            if target.is_none() {
                program_state = ProgramState::EndGoToPos;
            }
        }
        else if let Some(obst_bounds) = &last_obst_bounds {
            let route = calc_route(&car_center, &target.unwrap(), obst_bounds);
            draw_route_stream(&mut frame, &route.1)?;

            draw(&mut frame, target.unwrap(), (200.0, 50.0, 255.0))?;


            let next = next_point(&car_center, &route.1);

            // ---- Final-approach / suction-mouth tunables (CALIBRATE) ----
            // SUCK_OFFSET: px from car_center forward to the physical mouth,
            //   along the heading. MEASURE on the robot (blue marker drawn below).
            // SUCK_TOLERANCE: along-heading gap (px) at/below which SUCK fires
            //   once the mouth has reached OR PASSED the ball. Set ABOVE the
            //   per-frame creep travel so a frame cannot skip the trigger.
            // SUCK_LATERAL_TOLERANCE: max cross-track ball offset to allow SUCK.
            // APPROACH_DISTANCE: px straight-line car->ball range inside which we
            //   aim straight at the ball and bypass A*/obstacle weighting so a
            //   near-wall ball (~50px) stays reachable. Must exceed the planner's
            //   worst route-end (~146px measured) and its 80px early-exit.
            // CLOSE_RADIUS: interlock -- in Run, only honour close_enough when
            //   genuinely within this straight-line range, so a CAPPED A* route
            //   (route_lenth==0 while still far) cannot bee-line at the ball
            //   through obstacles.
            const SUCK_OFFSET: f32 = 70.0;
            const SUCK_TOLERANCE: f32 = 35.0;
            const SUCK_LATERAL_TOLERANCE: f32 = 40.0;
            const APPROACH_DISTANCE: i32 = 200;
            const CLOSE_RADIUS: i32 = 100;

            let dist_threshold = 1;
            // Når vi til sidst bare skal vende bagenden til, så sørger vi for den tror den er tæt nok på,
            // så den går direkte i gang med retningskalibrering.
            // route_close keeps the ORIGINAL meaning (route_lenth==0 or EndTurnAround).
            let route_close = next.route_lenth < dist_threshold || program_state == ProgramState::EndTurnAround;

            // Current heading vector (normalized once).
            let fx = (head.x - car_center.x) as f32;
            let fy = (head.y - car_center.y) as f32;
            let (fx, fy) = normalize(fx, fy);

            let ball = target.unwrap();

            // ---- ROBOT-MARKER HEIGHT-CORRECTION tunables (DEFAULT OFF) ----
            // MARKER_K == 1.0 disables the correction (identity), so the default
            // is ground-homography-only — a safe improvement needing zero extra
            // measurements. To enable: set MARKER_K = (H_cam - h)/H_cam (camera
            // height vs marker height, both above the floor; 0<k<1) and
            // MARKER_NADIR to the RECTIFIED image of the camera nadir (the floor
            // point directly under the lens) — see homography.rs::correct_marker.
            const MARKER_K: f64 = 1.0;
            const MARKER_NADIR: (f64, f64) = (0.0, 0.0);

            // RECTIFIED POSE: if the ground homography is calibrated, map the
            // ball (on the floor) and the robot markers into rectified ground
            // coordinates. The two markers are ELEVATED, so height-correct them
            // (identity while MARKER_K==1.0). If H is absent or ANY apply()
            // returns None (degenerate/horizon), rect_pose stays None and the
            // whole final approach falls back to the EXACT pixel path below.
            // Tuple: (ball, car_center, head) in rectified f32 pixels.
            let rect_pose: Option<((f32, f32), (f32, f32), (f32, f32))> = match &ground_h {
                Some(h) => {
                    let rb = h.apply(ball.x as f64, ball.y as f64);
                    let rc = h.apply(car_center.x as f64, car_center.y as f64);
                    let rh = h.apply(head.x as f64, head.y as f64);
                    match (rb, rc, rh) {
                        (Some(b), Some(c), Some(d)) => {
                            let c = homography::correct_marker(c, MARKER_NADIR, MARKER_K);
                            let d = homography::correct_marker(d, MARKER_NADIR, MARKER_K);
                            Some((
                                (b.0 as f32, b.1 as f32),
                                (c.0 as f32, c.1 as f32),
                                (d.0 as f32, d.1 as f32),
                            ))
                        }
                        _ => None,
                    }
                }
                None => None,
            };

            // Straight-line car->ball distance. PIXEL value (used for the pixel
            // fallback and unchanged when no rectified pose is available).
            let to_ball_sq = (ball.x - car_center.x).pow(2) + (ball.y - car_center.y).pow(2);

            // Rectified straight-line distance (f32) when available, else None.
            let to_ball_sq_rect: Option<f32> = rect_pose.map(|(b, c, _)| {
                let dx = b.0 - c.0;
                let dy = b.1 - c.1;
                dx * dx + dy * dy
            });

            // INTERLOCK: in the Run state a capped route can report
            // route_lenth==0 while the car is still far from the ball. Only
            // honour close_enough there when actually within CLOSE_RADIUS, so a
            // failed search never collapses into a straight bee-line through
            // obstacles. End* states keep the ORIGINAL route_close unchanged.
            // The CLOSE_RADIUS test uses rectified distance when available, else
            // the EXACT current integer pixel comparison.
            let within_close = match to_ball_sq_rect {
                Some(r) => r < (CLOSE_RADIUS * CLOSE_RADIUS) as f32,
                None => to_ball_sq < CLOSE_RADIUS * CLOSE_RADIUS,
            };
            let close_enough = if program_state == ProgramState::Run {
                route_close && within_close
            } else {
                route_close
            };

            // Final approach: Run-only, within APPROACH_DISTANCE. Aims straight
            // at the ball (bypassing the A* next point) so obstacle weighting
            // cannot bend the last leg -> a ~50px-from-wall ball is reachable.
            // Gate computed in RECTIFIED coords when available, else the EXACT
            // current integer pixel comparison.
            let within_approach = match to_ball_sq_rect {
                Some(r) => r < (APPROACH_DISTANCE * APPROACH_DISTANCE) as f32,
                None => to_ball_sq < APPROACH_DISTANCE * APPROACH_DISTANCE,
            };
            let final_approach = program_state == ProgramState::Run && within_approach;

            // Lost-direction guard: get_car_direction() returns Position{0,0}
            // (image ORIGIN), NOT car_center, when the marker is missing. Test
            // that sentinel on the PIXEL head so a lost heading can never trigger
            // SUCK on a garbage direction or project the mouth toward the origin.
            let head_valid = head.x != 0 || head.y != 0;

            // Project the physical mouth forward from car_center along heading.
            // PIXEL mouth, still drawn for the operator overlay regardless of H.
            let mouth = Position {
                x: car_center.x + (fx * SUCK_OFFSET) as i32,
                y: car_center.y + (fy * SUCK_OFFSET) as i32,
            };
            draw(&mut frame, mouth, (0.0, 0.0, 255.0))?;

            // AT-OR-BEYOND trigger: signed along-heading distance from the mouth
            // to the ball (positive = ball still ahead of the mouth) and the
            // perpendicular (cross-track) offset. SUCK fires when the mouth has
            // reached or passed the ball (forward_gap <= SUCK_TOLERANCE, allows
            // small negative = slight overshoot) and the lateral offset is
            // small. This cannot be skipped by a single fast frame: once the
            // mouth is at/after the ball the condition latches true instead of
            // flipping the heading and oscillating.
            //
            // RECTIFIED branch: when a rectified pose exists, compute the heading,
            // mouth, forward_gap and lateral entirely in rectified ground coords
            // so parallax cannot skew the suck decision anywhere on the field.
            // Otherwise fall back to the EXACT pixel expressions below.
            let (forward_gap, lateral) = match rect_pose {
                Some((b, c, d)) => {
                    let (rfx, rfy) = normalize(d.0 - c.0, d.1 - c.1);
                    let rmx = c.0 + rfx * SUCK_OFFSET;
                    let rmy = c.1 + rfy * SUCK_OFFSET;
                    let rbx = b.0 - rmx;
                    let rby = b.1 - rmy;
                    (rbx * rfx + rby * rfy, (rbx * rfy - rby * rfx).abs())
                }
                None => {
                    let bx = (ball.x - mouth.x) as f32;
                    let by = (ball.y - mouth.y) as f32;
                    (bx * fx + by * fy, (bx * fy - by * fx).abs())
                }
            };
            let on_ball = final_approach
                && head_valid
                && forward_gap <= SUCK_TOLERANCE
                && lateral <= SUCK_LATERAL_TOLERANCE;

            // In final approach (or End* close), aim DIRECTLY at the ball.
            let target_pos = if close_enough || final_approach {
                ball
            } else {
                next.pos
            };


            draw(&mut frame, target_pos, (200.0, 50.0, 255.0))?;


            // Desired direction
            let gx = (target_pos.x - car_center.x) as f32;
            let gy = (target_pos.y - car_center.y) as f32;

            let (gx, gy) = normalize(gx, gy);

            let angle = angle_between(fx, fy, gx, gy);

            let threshold = 7.0_f32; // originally 15.0

            // Enter the aligned-action block on the original close_enough (for
            // End* states this is the unchanged route_close) OR the Run final
            // approach. The final_approach term can NEVER trigger an End* arm
            // because it requires program_state == Run.
            let act = close_enough || final_approach;

            if program_state != ProgramState::Config {
                if angle.abs() > threshold.to_radians() {
                    if angle > 0.0 {
                        println!("TURN RIGHT");
                        send_command(&mut ev3, "right", &mut last_command);
                    } else {
                        println!("TURN LEFT");
                        send_command(&mut ev3, "left", &mut last_command);
                    }
                } else {
                    if act {
                        match program_state {
                            ProgramState::Run => {
                                if on_ball {
                                    println!("SUCK!");

                                    send_command(&mut pi, "start", &mut pi_last_command);
                                    send_command(&mut ev3, "stop", &mut last_command);
                                    std::thread::sleep(Duration::from_secs(3));

                                    send_command(&mut ev3, "backward", &mut last_command);
                                    std::thread::sleep(Duration::from_secs_f32(1.0));
                                    send_command(&mut ev3, "stop", &mut last_command);

                                    target = None;
                                } else {
                                    // PULSED creep: send_command latches, so
                                    // continuous "forward" runs the motor full
                                    // speed and a single accepted ~60px frame
                                    // could jump the SUCK window. Alternate
                                    // forward<->stop so per-frame travel stays
                                    // well below SUCK_TOLERANCE; the at-or-beyond
                                    // trigger then cannot be skipped.
                                    if last_command == "forward" {
                                        println!("CREEP-BRAKE");
                                        send_command(&mut ev3, "stop", &mut last_command);
                                    } else {
                                        println!("CREEP");
                                        send_command(&mut ev3, "forward", &mut last_command);
                                    }
                                }
                            }
                            ProgramState::EndGoToPos => {
                                send_command(&mut ev3, "stop", &mut pi_last_command);
                                program_state = ProgramState::EndOpen;
                            }
                            ProgramState::EndTurnAround => {
                                program_state = ProgramState::EndOpen;
                                send_command(&mut ev3, "stop", &mut pi_last_command);
                            }
                            _ => {}
                        }
                    }
                    else {
                        if program_state == ProgramState::EndGoToPos{
                            println!("BACKWARD");
                            send_command(&mut ev3, "backward", &mut last_command);
                        }
                        else {
                            println!("FORWARD");
                            send_command(&mut ev3, "forward", &mut last_command);
                        }
                    }
                }
            }
        }
        else {
            // target is set but no obstacle bounds have ever been detected (the
            // static walls/cross blob has been missing on every frame so far).
            // Skip routing/motion this frame instead of panicking; it self-heals
            // the moment the border is seen once, since bounds are cached.
            println!("waiting for obstacle detection...");
        }


        // Draw on screen :)
        for group in &state.groupings {
            for (pos, _) in &group.marks {
                let Rgb([r, g, b]) = group.color.rgb();
                imgproc::circle(
                    &mut frame,
                    core::Point::new(pos.x, pos.y),
                    2,
                    core::Scalar::new(b as f64, g as f64, r as f64, 0.0),
                    1,
                    imgproc::LINE_8,
                    0,
                )?;
            }
        }

        let elapsed = frame_start.elapsed();
        println!("frame took {}ms", elapsed.as_millis());

        if let Ok(true) = show_frame(&frame) {
            break;
        }
    }

    Ok(())
}



fn pythagoras(point1: &Position, point2: &Position) -> i32 {
    let dx = point2.x - point1.x;
    let dy = point2.y - point1.y;
    dx * dx + dy * dy
}

fn show_frame(frame: &Mat) -> opencv::Result<bool> {
    highgui::imshow("camera", frame)?;
    let key = highgui::wait_key(1)?;
    if key == 27 {
        return Ok(true);
    }

    Ok(false)
}



fn draw(mut frame: &mut Mat, pos: Position, color: (f64, f64, f64)) -> opencv::Result<()> {
    imgproc::circle(
        &mut frame,
        core::Point::new(pos.x, pos.y),
        30,
        core::Scalar::new(color.0, color.1, color.2, 0.0),
        30,
        imgproc::LINE_8,
        0,
    )?;

    Ok(())
}


impl Grouping {
    fn volume(&self) -> usize {
        self.marks.len()
    }

    fn center(&self) -> Position {
        // Pixel CENTROID (mean of all member pixels) instead of the AABB
        // midpoint. (max+min)/2 is set by the 2 extreme pixels, so a single
        // stray/outlier pixel or an asymmetric blob shifts the reported center
        // by tens of px -- the close-range aim error that makes the suction
        // miss. The mean averages over all N matched pixels, so the same stray
        // moves the center by only offset/N.
        let n = self.marks.len() as i64;
        if n == 0 {
            return Position { x: 0, y: 0 };
        }
        let (mut sx, mut sy) = (0i64, 0i64);
        for p in self.marks.keys() {
            sx += p.x as i64;
            sy += p.y as i64;
        }
        Position { x: (sx / n) as i32, y: (sy / n) as i32 }
    }
}

impl State {
    fn get_car_center(&mut self) -> Position {
        if let Some(group)= self.groupings.iter().find(|g| g.color == Color::CarCenter) {
            // calculate center
            group.center()
        }
        else {
            Position { x: 0, y: 0 } // TODO: Burde nok panikke
        }
    }

    fn get_car_direction(&mut self) -> Position {
        if let Some(group)= self.groupings.iter().find(|g| g.color == Color::CarDirection) {
            // calculate center
            group.center()
        }
        else {
            Position { x: 0, y: 0 }
        }
    }

    fn get_balls(&mut self) -> Vec<Position> {
        // let balls = self.groupings.iter().find(|g| g.color == Color::White).unwrap();
        // balls.marks.iter().map(|(pos, _)| pos.clone()).collect()
        let mut balls = Vec::new();
        //
        for group in &self.groupings {
            if group.color != Color::White { continue }
            balls.push(group.center());
        }


        balls
    }

    fn get_obstacles(&mut self) -> Option<Obstacles> {
        // groupings is sorted by descending volume in find_borders(): [0] is the
        // largest blob (walls), [1] is the second (cross). Return None instead
        // of panicking when a frame doesn't yield at least 2 obstacle groups
        // (glare/occlusion); the caller reuses the last good bounds.
        let walls: Vec<Position> = self.groupings.first()?.marks.keys().copied().collect();
        let cross: Vec<Position> = self.groupings.get(1)?.marks.keys().copied().collect();

        Some(Obstacles { walls, cross })
    }

    fn group_marks(&mut self) -> &mut Self {
        let mut taken: HashMap<Position, ()> = HashMap::new();

        for (start_pos, start_mark) in &self.marks {
            if taken.contains_key(start_pos) {
                continue;
            }

            let mut group = Grouping::default();
            group.color = start_mark.color;

            let mut queue = VecDeque::new();

            queue.push_back(start_pos.clone());
            taken.insert(start_pos.clone(), ());

            while let Some(pos) = queue.pop_front() {
                group.marks.insert(pos.clone(), false);

                // 8-connected flood fill
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }

                        let neighbor = Position {
                            x: pos.x + dx,
                            y: pos.y + dy,
                        };

                        if taken.contains_key(&neighbor) {
                            continue;
                        }

                        // Color-aware flood fill: only merge a neighbour that
                        // shares the SAME color as the seed. The previous test
                        // merged any 8-connected mark regardless of color, so a
                        // white ball touching the red wall/cross got absorbed
                        // into one grouping and its center() became a phantom
                        // point -- the robot then aimed short / into the wall.
                        // start_mark is in scope from the outer
                        // `for (start_pos, start_mark) in &self.marks` loop;
                        // Color is Copy so this is a shared borrow + copy.
                        match self.marks.get(&neighbor) {
                            Some(m) if m.color == start_mark.color => {}
                            _ => continue,
                        }

                        taken.insert(neighbor.clone(), ());
                        queue.push_back(neighbor);
                    }
                }
            }

            self.groupings.push(group);
        }

        self
    }

    /// Går igennem alle grupper og fjerner dem med for små volumener
    fn filter_groups(&mut self) -> &mut Self {
        self.groupings.retain(|g| {
            // let width = g.marks.keys().map(|p| p.x).max().unwrap_or(0) - g.marks.keys().map(|p| p.x).min().unwrap_or(0);
            let height = g.marks.keys().map(|p| p.y).max().unwrap_or(0) - g.marks.keys().map(|p| p.y).min().unwrap_or(0);

            if g.color == Color::White {
                g.volume() > 220 && height < 30
            }
            else {
                g.volume() > 220
            }

        });
        // self.groupings.retain(|g| g.color == Color::White && g.volume() < 1000 || g.color != Color::White);

        self
    }

    fn find_borders(&mut self) -> &mut Self {
        self.groupings.sort_by_key(|g| Reverse(g.volume()));

        // println!("Largest: {}", self.groupings.first().unwrap().volume());
        // println!("Cross: {}", self.groupings.iter().nth(1).unwrap().volume());

        self.find_map_border();
        self.find_cross();

        self
    }

    fn find_cross(&mut self) -> &mut Self {
        // Visualization-only (draws cross arm-tips into the working image). Bail
        // out drawing nothing, instead of panicking, when fewer than 2 groups
        // survive filtering on a frame.
        let group_to_inspect = match self.groupings.get(1) {
            Some(g) => g,
            None => return self,
        };

        let mut right = Position::new(0, 0);
        let mut left = Position::new(MAX, MAX);
        let mut top = Position::new(0, MAX);
        let mut bottom = Position::new(0, 0);

        for (pos, _) in &group_to_inspect.marks {
            if pos.y < top.y {
                top = pos.clone()
            }
            if pos.y > bottom.y {
                bottom = pos.clone()
            }

            if pos.x > right.x {
                right = pos.clone()
            }
            if pos.x < left.x {
                left = pos.clone()
            }
        }

        top.x = left.x + (right.x - left.x) / 2;
        bottom.x = left.x + (right.x - left.x) / 2;

        right.y = top.y + (bottom.y - top.y) / 2;
        left.y = top.y + (bottom.y - top.y) / 2;

        self.draw_mark(right.x as u32, right.y as u32, 10, Color::White);
        self.draw_mark(left.x as u32, left.y as u32, 10, Color::White);
        self.draw_mark(top.x as u32, top.y as u32, 10, Color::White);
        self.draw_mark(bottom.x as u32, bottom.y as u32, 10, Color::White);

        self
    }

    fn find_map_border(&mut self) -> &mut Self {
        // find den største

        let (width, height) = self.img.dimensions();
        let (width, height) = (width as i32, height as i32);

        // Visualization-only (draws border corners into the working image). Bail
        // out instead of panicking when no group exists on a frame.
        let group_to_inspect = match self.groupings.first() {
            Some(g) => g,
            None => return self,
        };

        // Det bagerste navn fortæller prioriteten.
        // Dvs. top_right er i top området og vi forsøger at finde den mest til højre
        let mut top_right = Position::new(0, height);
        let mut top_left = Position::new(width, height);

        let mut right_top = Position::new(0, height);
        let mut right_bottom = Position::new(0, 0);

        let mut left_top = Position::new(width, height);
        let mut left_bottom = Position::new(width, 0);

        let mut bottom_right = Position::new(0, 0);
        let mut bottom_left = Position::new(width, 0);

        // find the pixel most in the right corner and above half the image.
        for (pos, _) in &group_to_inspect.marks {
            if pos.x > top_right.x && pos.y < height / 4 {
                top_right = pos.clone();
            }
            if pos.x < top_left.x && pos.y < height / 4 {
                top_left = pos.clone();
            }
            if pos.y < right_top.y && pos.x > width / 4 * 3 {
                right_top = pos.clone();
            }
            if pos.y > right_bottom.y && pos.x > width / 4 * 3 {
                right_bottom = pos.clone();
            }
            if pos.y > left_bottom.y && pos.x < width / 4 {
                left_bottom = pos.clone();
            }
            if pos.y < left_top.y && pos.x < width / 4 * 3 {
                left_top = pos.clone();
            }

            if pos.x > bottom_right.x && pos.y > height / 4 * 3 {
                bottom_right = pos.clone()
            }
            if pos.x < bottom_left.x && pos.y > height / 4 * 3 {
                bottom_left = pos.clone()
            }
        }

        self.draw_mark(top_right.x as u32, top_right.y as u32, 10, Color::White);
        self.draw_mark(top_left.x as u32, top_left.y as u32, 10, Color::White);

        self.draw_mark(right_top.x as u32, right_top.y as u32, 10, Color::White);
        self.draw_mark(
            right_bottom.x as u32,
            right_bottom.y as u32,
            10,
            Color::White,
        );

        self.draw_mark(left_bottom.x as u32, left_bottom.y as u32, 10, Color::White);
        self.draw_mark(left_top.x as u32, left_top.y as u32, 10, Color::White);

        self.draw_mark(
            bottom_right.x as u32,
            bottom_right.y as u32,
            10,
            Color::White,
        );
        self.draw_mark(bottom_left.x as u32, bottom_left.y as u32, 10, Color::White);

        let y = left_bottom.y - (left_bottom.y - left_top.y) / 2;
        let x = left_bottom.x + (left_top.x - left_bottom.x) / 2;

        self.draw_mark(x as u32, y as u32, 10, Color::White);

        // HOMOGRAPHY CALIBRATION SOURCE: store the four band-filtered corners
        // [top_left, top_right, bottom_left, bottom_right] for main() to build
        // the ground homography from. Each was seeded to a frame-corner SENTINEL
        // (top_left=(width,height), top_right=(0,height), bottom_left=(width,0),
        // bottom_right=(0,0)); a corner still at its sentinel means NO detection
        // in its quarter-band, so we refuse to store a sentinel-laced set and
        // leave last_corners unchanged (main() then keeps the last good H or
        // falls back to pixel space). The corners are Copy, so copy them out
        // before assigning into self (no borrow conflict with group_to_inspect).
        let tl = top_left;
        let tr = top_right;
        let bl = bottom_left;
        let br = bottom_right;
        let tl_ok = !(tl.x == width && tl.y == height);
        let tr_ok = !(tr.x == 0 && tr.y == height);
        let bl_ok = !(bl.x == width && bl.y == 0);
        let br_ok = !(br.x == 0 && br.y == 0);
        if tl_ok && tr_ok && bl_ok && br_ok {
            self.last_corners = Some([tl, tr, bl, br]);
        }

        self
    }

    fn scan_for_marks(&mut self) -> &mut Self {
        let (width, height) = self.original_img.dimensions();

        let targets = self.targets.clone();

        for x in 0..width {
            for y in 0..height {
                for BetterTarget {
                    color,
                    target_hex,
                    precision,
                } in &targets.0
                {
                    let Rgb([r, g, b]) = self.original_img.get_pixel(x, y);
                    let Rgb([target_r, target_g, target_b]) = target_hex.rgb;

                    let (red_match, red_dist) = check_color(*r, target_r, *precision);
                    let (green_match, green_dist) = check_color(*g, target_g, *precision);
                    let (blue_match, blue_dist) = check_color(*b, target_b, *precision);

                    if red_match && green_match && blue_match {
                        self.marks.insert(
                            Position {
                                x: x as i32,
                                y: y as i32,
                            },
                            Mark {
                                precision: 0,
                                color: color.clone(),
                                x: x as i32,
                                y: y as i32,
                            },
                        );
                    }
                }
            }
        }

        return self;
    }

    fn draw_mark(&mut self, x: u32, y: u32, size: u32, color: Color) {
        let size = if x < size {
            x
        } else if y < size && x > y {
            y
        } else {
            size
        };

        let (w, h) = self.img.dimensions();

        for _x in x - size..x + size {
            for _y in y - size..y + size {
                if _x >= w || _y >= h {
                    continue;
                }

                self.img.put_pixel(_x, _y, color.rgb());
            }
        }
    }
}

/// calc distance from target to value
fn check_color(value: u8, target: u8, allowed_distance: u8) -> (bool, u32) {
    let value: i32 = value as i32;
    let target: i32 = target as i32;

    let dist = if value >= target {
        value - target
    } else {
        target - value
    };

    (dist <= allowed_distance as i32, dist as u32)
}

fn normalize(x: f32, y: f32) -> (f32, f32) {
    let len = (x*x + y*y).sqrt();
    if len == 0.0 {
        return (0.0, 0.0);
    }
    (x / len, y / len)
}

fn angle_between(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dot = ax * bx + ay * by;
    let cross = ax * by - ay * bx;
    cross.atan2(dot)
}
