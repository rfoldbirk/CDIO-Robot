use image::{ImageReader, Rgb, RgbImage};
use serde::Deserialize;
use std::{collections::HashMap, error::Error};

const RADIUS: i32 = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
struct Position {
    x: i32,
    y: i32,
}

enum Color {
    Ball,
    Car,
    Obstacle,
    Debug,
    RouteClosed,
    RouteOpen,
    RouteChosen,
}

struct ObstacleBounds {
    walls: Bounds,
    cross: Bounds,
}

#[derive(Debug)]
struct Bounds {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}



fn bounds(points: &[Position]) -> Bounds {
    Bounds {
        min_x: points.iter().map(|p| p.x).min().unwrap(),
        max_x: points.iter().map(|p| p.x).max().unwrap(),
        min_y: points.iter().map(|p| p.y).min().unwrap(),
        max_y: points.iter().map(|p| p.y).max().unwrap(),
    }
}

#[derive(Deserialize)]
struct Obstacles {
    walls: Vec<Position>,
    cross: Vec<Position>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let now = std::time::Instant::now();

    // Load the DATA so far :)
    let dyn_img = ImageReader::open("./scene.jpg")?.decode()?;
    let mut img: RgbImage = dyn_img.to_rgb8();

    let balls_json = std::fs::read_to_string("./balls.json")?;
    let obstacles_json = std::fs::read_to_string("./obstacles.json")?;

    let balls: Vec<Position> = serde_json::from_str(&balls_json)?;
    let obstacles: Obstacles = serde_json::from_str(&obstacles_json)?;

    let obst_bounds = ObstacleBounds {
        cross: bounds(&obstacles.cross),
        walls: bounds(&obstacles.walls),
    };

    let car: Position = serde_json::from_str(
        &std::fs::read_to_string("./car.json")?
    )?;
    draw_pixel(&mut img, &car, 40, Color::Car);

    // Calculate Route

    // find den tætteste og planlæg rute
    let ball = find_nearest_ball(&car, &balls).expect("Der burde være en bold!");
    draw_pixel(&mut img, &ball, 20, Color::Debug);
    let route = calc_route(&car, &ball, &obst_bounds);

    // draw cross
    // for x in obst_bounds.cross.min_x..=obst_bounds.cross.max_x {
    //     for y in obst_bounds.cross.min_y..=obst_bounds.cross.max_y {
    //         draw_pixel(&mut img, &Position { x, y }, 1, Color::Debug);
    //     }
    // }

    // draw_route_map_debug(&mut img, &route);
    draw_route_map_debug(&mut img, &route);
    draw_route(&mut img, &route.1);

    // draw balls
    for ball in &balls {
        draw_pixel(&mut img, ball, 10, Color::Ball);
    }

    img.save("map_computed.jpg")?;
    println!("executed in: {}ms", now.elapsed().as_millis());

    Ok(())
}

#[derive(Clone)]
struct Node {
    g_cost: i32, // distance from starting node
    h_cost: i32, // distance from end node
    parent: Option<Position>,
}

impl Node {
    fn f_cost(&self) -> i32 {
        self.g_cost + self.h_cost
    }
}

fn draw_route(img: &mut RgbImage, route: &HashMap<Position, Node>) {
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
        println!("Found closest!");
        draw_pixel(img, &pos, 20, Color::RouteOpen);

        let mut node: Node = route.get(&pos).unwrap().clone();
        while let Some(parent) = node.parent {
            draw_pixel(img, &parent, 7, Color::RouteChosen);
            node = route.get(&parent).unwrap().clone();
        }
    }
    else {
        println!("Could not find closest node?");
    }

}

fn get_first_node(route: &HashMap<Position, Node>) -> Option<Position> {
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

fn draw_route_map_debug(img: &mut RgbImage, route: &(HashMap<Position, Node>, HashMap<Position, Node>)) {
    for (pos, _) in &route.0 {
        draw_pixel(
            img,
            pos,
            3,
            Color::RouteOpen,
            // match node.open {
                // true => Color::RouteOpen,
                // false => Color::RouteClosed,
            // },
        );
    }
    for (pos, node) in &route.1 {
        draw_pixel(
            img,
            pos,
            3,
            Color::RouteClosed,
            // match node.open {
                // true => Color::RouteOpen,
                // false => Color::RouteClosed,
            // },
        );
    }
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

fn find_nearest_ball(car: &Position, balls: &Vec<Position>) -> Option<Position> {
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

fn draw_pixel(img: &mut RgbImage, pos: &Position, size: i32, color: Color) {
    for x in pos.x - size..pos.x + size {
        for y in pos.y - size..pos.y + size {
            // skip the pixels that are outside
            if x < 0 || y < 0 || x >= img.width() as i32 || y >= img.height() as i32 {
                continue;
            }

            img.put_pixel(
                x as u32,
                y as u32,
                match color {
                    Color::Ball => Rgb([200, 150, 100]),
                    Color::Car => Rgb([150, 200, 150]),
                    Color::Obstacle => Rgb([100, 150, 200]),
                    Color::Debug => Rgb([211, 176, 33]),
                    Color::RouteOpen => Rgb([192, 132, 252]),
                    Color::RouteClosed => Rgb([139, 92, 246]),
                    Color::RouteChosen => Rgb([192, 170, 252]),
                },
            );
        }
    }
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

    // Cross penalty SYNCED to the live analyzer (analyzer/src/path.rs): exponent
    // 7.8, weight 500000 -- the branch's own stronger anti-cross-hitting tuning,
    // kept so this offline route visualizer matches the hardware.
    (normalized.powf(7.8) * 500000.0) as i32
}
