use image::{Rgb, RgbImage, io::Reader as ImageReader};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::time::Duration;
use rand::{Rng, RngExt};

// State
struct State {
    original_img: RgbImage,
    img: RgbImage,
    color_target: Target,
    marks: HashMap<Position, Mark>,
    groupings: Vec<Grouping>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct Grouping(HashMap<Position, bool>);

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
struct Position {
    x: i32,
    y: i32,
}

struct Rectangle {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}



struct Target {
    white: Rgb<u8>,
    white_precision: u8,
    orange: Rgb<u8>,
    orange_precision: u8,
}

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
enum Color {
    White,
    Orange,
    Debug,
}

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
struct Mark {
    /// distance from target color
    precision: i32,
    color: Color,
    x: i32,
    y: i32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let now = std::time::Instant::now();
    let img_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "images/14.jpg".to_string());

    let wr: u8 = env::args()
        .nth(2)
        .unwrap_or("255".to_string())
        .parse()
        .unwrap_or(255);
    let wg: u8 = env::args()
        .nth(3)
        .unwrap_or("250".to_string())
        .parse()
        .unwrap_or(250);
    let wb: u8 = env::args()
        .nth(4)
        .unwrap_or("242".to_string())
        .parse()
        .unwrap_or(242);
    let or: u8 = env::args()
        .nth(5)
        .unwrap_or("239".to_string())
        .parse()
        .unwrap_or(239);
    let og: u8 = env::args()
        .nth(6)
        .unwrap_or("182".to_string())
        .parse()
        .unwrap_or(182);
    let ob: u8 = env::args()
        .nth(7)
        .unwrap_or("104".to_string())
        .parse()
        .unwrap_or(104);

    // precision values (unøjagtigheder)
    let wp: u8 = env::args()
        .nth(8)
        .unwrap_or("74".to_string())
        .parse()
        .unwrap_or(74);
    let op: u8 = env::args()
        .nth(9)
        .unwrap_or("41".to_string())
        .parse()
        .unwrap_or(41);

    println!("input: wr={wr}, wg={wg}, wb={wb}, or={or}, og={og}, ob={ob}, wp={wp}, op={op}");

    let dyn_img = ImageReader::open(&img_path)?.decode()?;
    let original_img: RgbImage = dyn_img.to_rgb8();
    let img = original_img.clone();
    // let (width, height) = original_img.dimensions();

    let mut state = State {
        original_img,
        img,
        color_target: Target {
            white: Rgb([wr, wg, wb]),
            white_precision: wp,
            orange: Rgb([or, og, ob]),
            orange_precision: op,
        },
        marks: HashMap::new(),
        groupings: Vec::new(),
    };

    state
        .scan_for_marks() // finder alle pixels der har samme farve som de valgte farver - eller i det mindste er tæt nok på
        .save().wait()
        .group_marks() // grupperer pixels som ligger op ad hinanden
        .save();

    println!("executed in: {}ms", now.elapsed().as_millis());
    Ok(())
}



impl Grouping {
    fn calculate_center(&self) -> Rectangle {
        todo!();
        // calculate center from points
        // let x_min = self.0.iter().map(|m| m.x).min().unwrap_or(0);
        // let x_max = self.0.iter().map(|m| m.x).max().unwrap_or(0);
        // let y_min = self.0.iter().map(|m| m.y).min().unwrap_or(0);
        // let y_max = self.0.iter().map(|m| m.y).max().unwrap_or(0);

        // Rectangle {
        //     x: x_min,
        //     y: y_min,
        //     width: x_max - x_min,
        //     height: y_max - y_min,
        // }
    }

    fn distance(&self, other: &Grouping) -> f64 {
        let center1 = self.calculate_center();
        let center2 = other.calculate_center();

        ((center1.x - center2.x) as f64).abs() + ((center1.y - center2.y) as f64).abs()
    }
}

impl State {
    fn save(&mut self) -> &mut Self {
        match self.img.save("out.png") {
            Ok(_) => {}
            Err(e) => panic!("failed to save image: {}", e),
        }

        self
    }

    fn clear_image(&mut self) -> &mut Self {
        self.img = self.original_img.clone();
        self.save()
    }

    fn wait(&mut self) -> &mut Self {
        std::thread::sleep(Duration::from_millis(1000));
        self
    }

    fn capture(&mut self) -> &mut Self {
        self.save().clear_image().wait()
    }

    fn group_marks(&mut self) -> &mut Self {
        let mut taken: HashMap<Position, ()> = HashMap::new();

        for (pos, mark) in &self.marks {
            if taken.contains_key(pos) { continue }

            let mut group = Grouping::default();
            group.0.insert(pos.clone(), true);

            let mut count = 0;

            loop {
                count += 1;
                if count > 10000 { break }
                // gå alle igennem, som ikke er blevet udforsket!
                let mut insert_later: Vec<Position> = Vec::new();
                let mut did_something = false;

                for (pos, needs_searching) in group.0.iter_mut() {
                    if !(*needs_searching) { continue }
                    println!("Searching {pos:?}");
                    did_something = true;
                    *needs_searching = false;

                    let mut new_positions = Vec::new();

                    for i in 0..8 {
                        new_positions.push(Position { x: pos.x, y: pos.y - i });
                        new_positions.push(Position { x: pos.x, y: pos.y + i });
                        new_positions.push(Position { x: pos.x - i, y: pos.y });
                        new_positions.push(Position { x: pos.x + i, y: pos.y });
                    }

                    // filtrer positioner som allerede er blevet set.
                    for new_pos in new_positions {
                        if taken.contains_key(&new_pos) { continue }
                        if !self.marks.contains_key(&new_pos) { continue }
                        insert_later.push(new_pos);
                    }

                }

                for key in insert_later {
                    group.0.insert(key.clone(), true);
                    taken.insert(key.clone(), ());
                }

                if !did_something { break }
            }

            self.groupings.push(group);
        }

        println!("amount of groups: {}", self.groupings.len());


        // self.clear_image();

        let mut offset: i32 = 50;
        for g in &self.groupings {
            let mut rng = rand::rng();
            let rgb: [u8; 3] = rng.random();

            for (p, searched) in g.0.iter() {
                self.img.put_pixel(p.x as u32, p.y as u32, Rgb(rgb));
            }

        }

        self
    }

    fn scan_for_marks(&mut self) -> &mut Self {
        let (width, height) = self.original_img.dimensions();

        for x in 0..width {
            for y in 0..height {
                let Rgb([r, g, b]) = self.original_img.get_pixel(x, y);
                let Rgb([wr, wg, wb]) = self.color_target.white;
                let Rgb([or, og, ob]) = self.color_target.orange;

                let wp = self.color_target.white_precision;
                let op = self.color_target.orange_precision;

                let (red_match, red_dist) = check_color(*r, wr, wp);
                let (green_match, green_dist) = check_color(*g, wg, wp);
                let (blue_match, blue_dist) = check_color(*b, wb, wp);
                let (orange_red_match, orange_red_dist) = check_color(*r, or, op);
                let (orange_green_match, orange_green_dist) = check_color(*g, og, op);
                let (orange_blue_match, orange_blue_dist) = check_color(*b, ob, op);

                // hvis farverne matcher eksakt så tilføjer vi dem
                if red_match && green_match && blue_match {
                    self.marks.insert(
                        Position {
                            x: x as i32,
                            y: y as i32,
                        },
                        Mark {
                            precision: 0,
                            color: Color::White,
                            x: x as i32,
                            y: y as i32,
                        },
                    );
                    self.draw_mark(x, y, 1, Color::White);
                }

                // hvis farverne matcher eksakt så tilføjer vi dem
                if orange_red_match && orange_green_match && orange_blue_match {
                    println!("Putting White Mark on ({x}, {y})");
                    self.marks.insert(
                        Position {
                            x: x as i32,
                            y: y as i32,
                        },
                        Mark {
                            precision: 0,
                            color: Color::Orange,
                            x: x as i32,
                            y: y as i32,
                        },
                    );
                    // self.img.put_pixel(x, y, Rgb([200, 200, 250]));
                    self.draw_mark(x, y, 1, Color::Orange);
                }
            }
        }

        return self;
    }

    fn draw_rectangle(&mut self, rect: &Rectangle, color: Color) {
        for x in rect.x..rect.x + rect.width {
            for y in rect.y..rect.y + rect.height {
                // only draw the border
                if x == rect.x
                    || x == rect.x + rect.width - 1
                    || y == rect.y
                    || y == rect.y + rect.height - 1
                {
                    self.img.put_pixel(
                        x as u32,
                        y as u32,
                        match color {
                            Color::White => Rgb([200, 200, 250]),
                            Color::Orange => Rgb([255, 165, 0]),
                            Color::Debug => Rgb([250, 133, 152]),
                        },
                    );
                }
            }
        }
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

                self.img.put_pixel(
                    _x,
                    _y,
                    match color {
                        Color::White => Rgb([200, 200, 250]),
                        Color::Orange => Rgb([255, 165, 0]),
                        Color::Debug => Rgb([222, 193, 132]),
                    },
                );
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

// fn color_diff(state: &State, color: Rgb<u8>) -> u32 {
//     let delta_r = color.0[0].abs_diff(state.global_avg.0[0]);
//     let delta_g = color.0[1].abs_diff(state.global_avg.0[1]);
//     let delta_b = color.0[2].abs_diff(state.global_avg.0[2]);
//     (delta_r as u32).pow(2) + (delta_g as u32).pow(2) + (delta_b as u32).pow(2)
// }
