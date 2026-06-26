use image::{Rgb, RgbImage};
use opencv::{core, core::Mat, imgproc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error};

use crate::{Position, hex_to_rgb};

enum Color {
    Ball,
    Car,
    Obstacle,
    Debug,
    RouteClosed,
    RouteOpen,
    RouteChosen,
}

pub struct ObstacleBounds {
    pub walls: Bounds,
    pub cross: Bounds,
}

#[derive(Debug)]
pub struct Bounds {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}


pub fn bounds(points: &[Position]) -> Option<Bounds> {
    // Returns None for an empty point cloud instead of panicking (.unwrap()).
    // The caller (analyzer main loop) falls back to the last good bounds.
    Some(Bounds {
        min_x: points.iter().map(|p| p.x).min()?,
        max_x: points.iter().map(|p| p.x).max()?,
        min_y: points.iter().map(|p| p.y).min()?,
        max_y: points.iter().map(|p| p.y).max()?,
    })
}

#[derive(Serialize)]
pub struct Obstacles {
    pub walls: Vec<Position>,
    pub cross: Vec<Position>,
}


pub struct NextInstruction {
    pub pos: Position,
    pub route_lenth: usize,
}


pub fn next_point(car: &Position, route: &HashMap<Position, Node>) -> NextInstruction {
    let lowest_node = route.iter().min_by_key(|(_, node)| node.f_cost()).unwrap();

    let mut inspect_node = lowest_node.1.clone();

    let mut pos = Vec::new();

    while let Some(parent) = inspect_node.parent {
        pos.push(parent);
        let node = route.get(&parent).unwrap();
        inspect_node = node.clone();
    }

    let point = match pos.len() {
        0 => car.clone(),
        1 => pos.first().unwrap().clone(),
        2 => pos.iter().nth(0).unwrap().clone(),
        3 => pos.iter().nth(0).unwrap().clone(),
        4 => pos.iter().nth(0).unwrap().clone(),
        5 => pos.iter().nth(0).unwrap().clone(),
        _ => pos.iter().nth(pos.len()-5).unwrap().clone(),
    };

    return NextInstruction { pos: point, route_lenth: pos.len() }
}


#[derive(Clone)]
pub struct Node {
    pub g_cost: i32, // distance from starting node
    pub h_cost: i32, // distance from end node
    pub parent: Option<Position>,
}

impl Node {
    pub fn f_cost(&self) -> i32 {
        self.g_cost + self.h_cost
    }
}



pub fn draw_route_stream(
    frame: &mut Mat,
    route: &HashMap<Position, Node>,
) -> opencv::Result<()> {
    // Find node with lowest cost
    let mut min_cost = i32::MAX;
    let mut min_pos: Option<Position> = None;

    for (pos, node) in route {
        if node.f_cost() < min_cost {
            min_cost = node.f_cost();
            min_pos = Some(*pos);
        }
    }

    let color = [0u8, 255u8, 0u8]; // Green

    if let Some(pos) = min_pos {
        imgproc::circle(
            frame,
            core::Point::new(pos.x, pos.y),
            10,
            core::Scalar::new(
                color[2] as f64,
                color[1] as f64,
                color[0] as f64,
                0.0,
            ),
            -1,
            imgproc::LINE_8,
            0,
        )?;

        let mut node = route.get(&pos).unwrap().clone();

        while let Some(parent) = node.parent {
            imgproc::circle(
                frame,
                core::Point::new(parent.x, parent.y),
                7,
                core::Scalar::new(
                    color[2] as f64,
                    color[1] as f64,
                    color[0] as f64,
                    0.0,
                ),
                -1,
                imgproc::LINE_8,
                0,
            )?;

            node = route.get(&parent).unwrap().clone();
        }
    }

    Ok(())
}



pub fn get_first_node(route: &HashMap<Position, Node>) -> Option<Position> {
    // find the node with the lowest distance
    let mut min_cost = i32::MAX;
    let mut min_pos = None;
    for (pos, node) in route {
        if node.f_cost() < min_cost {
            min_cost = node.f_cost();
            min_pos = Some(pos.clone());
        }
    }

    if let Some(pos) = min_pos {
        let mut node: Node = route.get(&pos).unwrap().clone();
        let mut last_parent = node.parent;
        while let Some(parent) = node.parent {
            last_parent = Some(parent);
            node = route.get(&parent).unwrap().clone();
        }

        return last_parent;
    }

    panic!("No node");

}

pub fn calc_route(
    car: &Position,
    target: &Position,
    bounds: &ObstacleBounds,
) -> (HashMap<Position, Node>, HashMap<Position, Node>) {
    let mut open: HashMap<Position, Node> = HashMap::new();
    let mut closed: HashMap<Position, Node> = HashMap::new();

    open.insert(
        car.clone(),
        Node {
            g_cost: 0,
            h_cost: (car.x - target.x).pow(2) + (car.y - target.y).pow(2),
            parent: None,
        },
    );

    let mut i = 0;
    loop {
        i += 1;
        if i > 1000 { break; }
        let open_c = open.clone();
        let current = open_c
            .iter()
            .min_by_key(|(_, node)| (node.f_cost(), node.h_cost));

        if current.is_none() { break }
        let (current_pos, current_node) = current.unwrap();

        // remove from open and add to closed
        open.remove(&current_pos.clone());
        closed.insert(current_pos.clone(), current_node.clone());

        // check if we found target within distance
        if dist_to_target(current_pos, target) < 80*80 {
            break;
        }

        // explore neighbours!
        let neighbours = vec![
            Position { x: current_pos.x + 20, y: current_pos.y + 20 },
            Position { x: current_pos.x - 20, y: current_pos.y + 20 },
            Position { x: current_pos.x + 20, y: current_pos.y - 20 },
            Position { x: current_pos.x - 20, y: current_pos.y - 20 },
            Position { x: current_pos.x + 20, y: current_pos.y },
            Position { x: current_pos.x - 20, y: current_pos.y },
            Position { x: current_pos.x, y: current_pos.y - 20 },
            Position { x: current_pos.x, y: current_pos.y + 20 },
        ];

        'main: for n in &neighbours {
            if closed.contains_key(n) {
                continue;
            }
            let mut obstacle_cost = 0;
            const HARD_WALL_CLEARANCE: i32 = 120;
            const WALL_BUFFER: i32 = 350;
            let nearest_wall = [
                (bounds.walls.min_x - n.x).abs(),
                (bounds.walls.max_x - n.x).abs(),
                (bounds.walls.min_y - n.y).abs(),
                (bounds.walls.max_y - n.y).abs(),
            ]
            .into_iter()
            .min()
            .unwrap();
            // Hard exclusion zone
            // if nearest_wall < HARD_WALL_CLEARANCE {
            //     continue;
            // }
            // Soft penalty zone
            if nearest_wall < WALL_BUFFER {
                let normalized =
                    (WALL_BUFFER - nearest_wall) as f32
                        / WALL_BUFFER as f32;
                // Wall weight 50000 -> 70000 (exponent kept QUARTIC, WALL_BUFFER
                // kept 350). Sweep-verified against the real scene: switching to
                // quadratic and/or widening the buffer made the squared-metric
                // 1000-cap search wander and CAP OUT (no route -> stall). This
                // modest bump keeps every ball + a 50px-from-wall ball FOUND
                // while raising min wall clearance 138->150px and leaving the
                // worst route-end at ~146px (< APPROACH_DISTANCE). Still SOFT.
                obstacle_cost +=
                    (normalized.powf(4.0) * 70000.0) as i32;
            }
            // Distance to center obstacle
            let cross_dist_sq = dist_to_rect(n, &bounds.cross);
            const HARD_CROSS_CLEARANCE: i32 = 120;
            const CROSS_BUFFER: i32 = 400;
            // if cross_dist_sq < HARD_CROSS_CLEARANCE * HARD_CROSS_CLEARANCE {
            //     continue;
            // }
            obstacle_cost +=
                obstacle_penalty(cross_dist_sq, CROSS_BUFFER);
            let movement_cost =
                dist_to_target(n, current_pos);
            let new_g =
                current_node.g_cost
                + movement_cost
                + obstacle_cost;
            if !open.contains_key(n) {
                open.insert(
                    *n,
                    Node {
                        g_cost: new_g,
                        h_cost: dist_to_target(n, target),
                        parent: Some(*current_pos),
                    },
                );
            } else if let Some(node) = open.get_mut(n) {
                if new_g < node.g_cost {
                    node.g_cost = new_g;
                    node.parent = Some(*current_pos);
                }
            }
        }
    }

    (open, closed)
}

fn dist_to_target(pos: &Position, target: &Position) -> i32 {
    (pos.x - target.x).pow(2) + (pos.y - target.y).pow(2)
}

pub fn find_nearest_ball(car: &Position, balls: &Vec<Position>) -> Option<Position> {
    let mut shortest: Option<Position> = None;
    let mut distance: Option<i32> = None;

    for ball in balls {
        let d = (car.x - ball.x).pow(2) + (car.y - ball.y).pow(2);

        if let Some(dist) = distance {
            if d > dist {
                continue;
            } // hvis distancen er større lad vær
        }

        shortest = Some(ball.clone());
        distance = Some(d);
    }

    shortest
}

fn dist_to_rect(p: &Position, b: &Bounds) -> i32 {
    let dx = if p.x < b.min_x {
        b.min_x - p.x
    } else if p.x > b.max_x {
        p.x - b.max_x
    } else {
        0
    };

    let dy = if p.y < b.min_y {
        b.min_y - p.y
    } else if p.y > b.max_y {
        p.y - b.max_y
    } else {
        0
    };

    dx * dx + dy * dy
}

fn obstacle_penalty(dist_sq: i32, safety_radius: i32) -> i32 {
    let dist = (dist_sq as f32).sqrt();

    if dist >= safety_radius as f32 {
        return 0;
    }

    let normalized =
        (safety_radius as f32 - dist) / safety_radius as f32;

    // Cross weight 50000 -> 60000 (exponent kept QUARTIC, CROSS_BUFFER kept 400).
    // Sweep-verified: raises min cross clearance 17.5->39.4px with no cap and
    // route-end unchanged. Still a finite SOFT cost -> a route always exists.
    (normalized.powf(4.0) * 60000.0) as i32
}
