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

#[derive(Clone, Copy)]
struct Circle {
    center: Position,
    radius: i32,
}

impl Circle {
    fn new(pos: Position) -> Circle {
        Circle {
            center: pos,
            radius: RADIUS,
        }
    }

    fn contains_with_clearance(&self, point: Position, clearance: i32) -> bool {
        let dx = point.x - self.center.x;
        let dy = point.y - self.center.y;
        let allowed_radius = self.radius + clearance;

        dx * dx + dy * dy <= allowed_radius * allowed_radius
    }

    fn draw(&self, img: &mut RgbImage) {
        let img_w = img.width() as i32;
        let img_h = img.height() as i32;

        // Iterate horizontally across the circle and compute the corresponding y offsets
        for dx in -self.radius..=self.radius {
            let x = self.center.x + dx;
            if x < 0 || x >= img_w {
                continue;
            }

            // Compute squared vertical distance from circle equation: y_offset^2 = r^2 - dx^2
            let sq = self.radius * self.radius - dx * dx;
            if sq < 0 {
                continue;
            }

            // sqrt -> vertical offset (rounded)
            let dy = (f64::from(sq)).sqrt().round() as i32;

            let y_top = self.center.y - dy;
            let y_bottom = self.center.y + dy;

            if y_top >= 0 && y_top < img_h {
                draw_pixel(img, &Position { x, y: y_top }, 3, Color::Obstacle);
            }
            if y_bottom >= 0 && y_bottom < img_h {
                draw_pixel(img, &Position { x, y: y_bottom }, 3, Color::Obstacle);
            }
        }
    }
}

fn is_too_close(point: Position, circles: &[Circle], clearance: i32) -> bool {
    circles
        .iter()
        .any(|circle| circle.contains_with_clearance(point, clearance))
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
    let dyn_img = ImageReader::open("./map.jpg")?.decode()?;
    let mut img: RgbImage = dyn_img.to_rgb8();

    let balls_json = std::fs::read_to_string("./balls.json")?;
    let obstacles_json = std::fs::read_to_string("./obstacles.json")?;

    let balls: Vec<Position> = serde_json::from_str(&balls_json)?;
    let obstacles: Obstacles = serde_json::from_str(&obstacles_json)?;

    let obst_bounds = ObstacleBounds {
        cross: bounds(&obstacles.cross),
        walls: bounds(&obstacles.walls),
    };

    let car = Position { x: 1200, y: 400 };
    draw_pixel(&mut img, &car, 20, Color::Car);

    // Calculate Route

    // find den tætteste og planlæg rute
    let ball = find_nearest_ball(&car, &balls).expect("Der burde være en bold!");
    draw_pixel(&mut img, &ball, 20, Color::Debug);
    let route = calc_route(&car, &ball, &obst_bounds);
    draw_route_map_debug(&mut img, &route);
    draw_route(&mut img, &route.1);

    // draw balls
    for ball in &balls {
        draw_pixel(&mut img, ball, 10, Color::Ball);
    }

    img.save("map_computed.jpg")?;

    println!(
        "vol: {} and {}, amount of balls: {}",
        obstacles.walls.len(),
        obstacles.cross.len(),
        balls.len()
    );
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
        draw_pixel(img, &pos, 10, Color::RouteChosen);

        let mut node: Node = route.get(&pos).unwrap().clone();
        while let Some(parent) = node.parent {
            draw_pixel(img, &parent, 7, Color::RouteChosen);
            node = route.get(&parent).unwrap().clone();
        }
    }

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

fn calc_route(
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


        println!("Looking at: {:?}", current_pos);

        // remove from open and add to closed
        open.remove(&current_pos.clone());
        closed.insert(current_pos.clone(), current_node.clone());

        println!("F cost: {}", dist_to_target(current_pos, target));

        // check if we found target within distance
        if dist_to_target(current_pos, target) < 100*100 {
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
            if closed.contains_key(n) { continue } // check if neighbour is in closed

            // TODO: check if node is too close to obstacles!
            let dists_to_check = vec![
                (bounds.walls.max_x-100) - n.x,
                (bounds.walls.min_y+100) - n.y,
            ];

            for dist in dists_to_check {
                if dist.abs() < 100 {
                    // println!("dist: {}", dist.abs());
                    continue 'main;
                }
            }

            // Check distance to cross
            if n.x >= bounds.cross.min_x-100 && n.x <= bounds.cross.max_x+100 && n.y >= bounds.cross.min_y-100 && n.y <= bounds.cross.max_y+100 {
                continue;
            }

            if !open.contains_key(n) {
                let new_node = Node {
                    g_cost: current_node.g_cost + dist_to_target(n, current_pos),
                    h_cost: dist_to_target(n, target),
                    parent: Some(current_pos.clone()),
                };

                open.insert(*n, new_node);
            }
            else if let Some(node) = open.get_mut(n) {
                let extra_dist = dist_to_target(n, current_pos);
                let f_cost = current_node.f_cost() + extra_dist;

                if f_cost < node.f_cost() {
                    node.parent = Some(current_pos.clone());
                    node.g_cost = current_node.g_cost + extra_dist;
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
