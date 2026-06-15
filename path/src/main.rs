use std::{collections::{HashMap, HashSet, VecDeque}, error::Error};

use image::{ImageReader, Rgb, RgbImage};
use serde::Deserialize;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
struct Position {
    x: i32,
    y: i32,
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


    let car = Position {
        x: 1500,
        y: 400,
    };

    if let Some(path) = shortest_path_to_ball(car, &balls, &obstacles) {
        println!("Path length: {}", path.len());

        for p in &path {
            println!("{}, {}", p.x, p.y);
            img.put_pixel(p.x as u32, p.y as u32, Rgb([200, 200, 255]));
        }
    } else {
        println!("No reachable ball");
    }

    img.save("map_computed.jpg")?;

    println!("vol: {} and {}, amount of balls: {}", obstacles.walls.len(), obstacles.cross.len(), balls.len());
    println!("executed in: {}ms", now.elapsed().as_millis());

    Ok(())
}

fn expand_obstacles(
    obstacles: &std::collections::HashSet<Position>,
    clearance: i32,
) -> std::collections::HashSet<Position> {
    let mut expanded = obstacles.clone();

    for obstacle in obstacles {
        for dx in -clearance..=clearance {
            for dy in -clearance..=clearance {

                // circular expansion
                if dx * dx + dy * dy > clearance * clearance {
                    continue;
                }

                expanded.insert(Position {
                    x: obstacle.x + dx,
                    y: obstacle.y + dy,
                });
            }
        }
    }

    expanded
}

fn shortest_path_to_ball(
    start: Position,
    balls: &[Position],
    obstacles: &Obstacles,
) -> Option<Vec<Position>> {

    let raw_obstacles: HashSet<Position> = obstacles
        .walls
        .iter()
        .chain(obstacles.cross.iter())
        .copied()
        .collect();

    let blocked = expand_obstacles(&raw_obstacles, 20);

    let balls: HashSet<Position> = balls.iter().copied().collect();

    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();

    // child -> parent
    let mut parent = HashMap::<Position, Position>::new();

    queue.push_back(start);
    visited.insert(start);

    while let Some(current) = queue.pop_front() {

        // Found closest ball
        if balls.contains(&current) {
            let mut path = vec![current];

            let mut p = current;

            while let Some(prev) = parent.get(&p) {
                path.push(*prev);
                p = *prev;
            }

            path.reverse();
            return Some(path);
        }

        let neighbors = [
            Position { x: current.x + 1, y: current.y },
            Position { x: current.x - 1, y: current.y },
            Position { x: current.x, y: current.y + 1 },
            Position { x: current.x, y: current.y - 1 },
        ];

        for next in neighbors {
            if blocked.contains(&next) {
                continue;
            }

            if visited.insert(next) {
                parent.insert(next, current);
                queue.push_back(next);
            }
        }
    }

    None
}


