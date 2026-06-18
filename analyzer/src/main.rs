use image::{Rgb, RgbImage, io::Reader as ImageReader};
use serde::Serialize;
use std::{cmp::Reverse, collections::HashMap};
use std::env;
use std::error::Error;
use std::time::Duration;
use rand::{Rng, RngExt};

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
    color_target: Target,
    targets: Targets,
    marks: HashMap<Position, Mark>,
    groupings: Vec<Grouping>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct Grouping {
    marks: HashMap<Position, bool>,
    color: Color,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize)]
struct Position {
    x: i32,
    y: i32,
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
            Color::White => Rgb([200, 200, 250]),
            Color::Orange => Rgb([255, 165, 0]),
            Color::Red => Rgb([255, 125, 20]),
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

        Self {
            hex,
            rgb,
        }
    }
}


#[derive(Debug, Clone)]
struct Targets(Vec<BetterTarget>);

struct Target {
    white: Rgb<u8>,
    white_precision: u8,
    orange: Rgb<u8>,
    orange_precision: u8,
    red: Rgb<u8>,
    red_precision: u8,
}

#[derive(Default, Hash, PartialEq, Eq, Debug, Clone, Copy)]
enum Color {
    #[default]
    White,
    Orange,
    Red,
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

// enum Command {
//     GetMap,
// }


// impl Command {
//     fn parse(command: &str) -> Self {
//         match command {
//             "map" => Self::GetMap,
//             _ => panic!("Unsupported command: {command}"),
//         }
//     }
// }


fn main() -> Result<(), Box<dyn Error>> {
    let now = std::time::Instant::now();
    let img_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "images/14.jpg".to_string());

    // load farve værdier fra kommandolinjen
    let white_hex: String = env::args().nth(2).unwrap_or("#FFFFFF".to_string());
    let orange_hex: String = env::args().nth(3).unwrap_or("#DB2535".to_string());
    let red_hex: String = env::args().nth(4).unwrap_or("#D82938".to_string());

    // precision values (unøjagtigheder)
    let wp: u8 = env::args().nth(5).unwrap_or("74".to_string()).parse().expect("Naah");
    let op: u8 = env::args().nth(6).unwrap_or("41".to_string()).parse().expect("Naah");
    let rp: u8 = env::args().nth(7).unwrap_or("41".to_string()).parse().expect("Naah");

    // let command: String = env::args().nth(8).unwrap_or("all".into()).parse().unwrap();
    // let command: Command = Command::parse(&command);

    // print så vi kan se hvad fanden der foregår :)
    println!("INPUT: {white_hex}, {orange_hex}, {red_hex}, PRECISION: {wp}, {op}, {rp}");

    let white = hex_to_rgb(&white_hex);
    let orange = hex_to_rgb(&orange_hex);
    let red = hex_to_rgb(&red_hex);




    let dyn_img = ImageReader::open(&img_path)?.decode()?;
    let original_img: RgbImage = dyn_img.to_rgb8();
    let img = original_img.clone();


    
    let mut state = State {
        original_img,
        img,
        color_target: Target {
            white,
            white_precision: wp,
            orange,
            orange_precision: op,
            red,
            red_precision: rp, 
        },
        targets: Targets(vec![
            BetterTarget { color: Color::White, target_hex: Hex::new(white_hex), precision: wp },
            BetterTarget { color: Color::Orange, target_hex: Hex::new(orange_hex), precision: op },
            BetterTarget { color: Color::Red, target_hex: Hex::new(red_hex), precision: rp },
        ]),
        marks: HashMap::new(),
        groupings: Vec::new(),
    };


   
    state
        .scan_for_marks() // finder alle pixels der har samme farve som de valgte farver - eller i det mindste er tæt nok på
        .group_marks() // grupperer pixels som ligger op ad hinanden
        .filter_groups()
        .find_borders();


    state.draw_mark(1500, 400, 10, Color::Debug);

    state.save();
    
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

    fn volume(&self) -> usize {
        self.marks.len()
    }

    fn distance(&self, other: &Grouping) -> f64 {
        let center1 = self.calculate_center();
        let center2 = other.calculate_center();

        ((center1.x - center2.x) as f64).abs() + ((center1.y - center2.y) as f64).abs()
    }
}

impl State {
    fn save(&mut self) -> &mut Self {
        let walls: Vec<Position> = self.groupings.first().unwrap().marks.clone().into_keys().collect();
        let cross: Vec<Position> = self.groupings.iter().nth(1).unwrap().marks.clone().into_keys().collect();

        let output = Output {
            walls,
            cross,
        };

        let json = serde_json::to_string_pretty(&output).expect("du en taber");
        std::fs::write("./obstacles.json", json).expect("Nicklas har også en lille diller");

        match self.img.save("out.png") {
            Ok(_) => {}
            Err(e) => panic!("failed to save image: {}", e),
        }

        self
    }

    fn clear_image(&mut self) -> &mut Self {
        self.img = self.original_img.clone();
        self
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
            group.marks.insert(pos.clone(), true);
            group.color = mark.color;

            let mut count = 0;

            loop {
                count += 1;
                if count > 10000 { break }
                // gå alle igennem, som ikke er blevet udforsket!
                let mut insert_later: Vec<Position> = Vec::new();
                let mut did_something = false;

                for (pos, needs_searching) in group.marks.iter_mut() {
                    if !(*needs_searching) { continue }
                    // println!("Searching {pos:?}");
                    did_something = true;
                    *needs_searching = false;

                    let mut new_positions = Vec::new();

                    // Search in every position!
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
                    group.marks.insert(key.clone(), true);
                    taken.insert(key.clone(), ());
                }

                if !did_something { break }
            }

            self.groupings.push(group);
        }

        println!("amount of groups: {}", self.groupings.len());


        // self.clear_image();

        // let mut offset: i32 = 50;
        // for g in &self.groupings {
        //     let mut rng = rand::rng();
        //     let rgb: [u8; 3] = rng.random();

        //     for (p, searched) in g.0.iter() {
        //         self.img.put_pixel(p.x as u32, p.y as u32, Rgb(rgb));
        //     }
        // }

        self
    }


    /// Går igennem alle grupper og fjerner dem med for små volumener
    fn filter_groups(&mut self) -> &mut Self {
        // let mut groups_to_render = Vec::new();

        self.groupings.retain(|g| g.volume() > 200 );
        
        for group in &self.groupings.clone() {
            if group.color == Color::Red && group.volume() < 15000 { continue }
            
            self.draw_group(group);
            println!("{:#?} Group has volume: {}", group.color, group.volume());
        }


        // let group = self.groupings.iter().nth(4);
        // let group = group.unwrap().clone();
        // self.draw_group(&group);
        

        self
    }

    fn find_borders(&mut self) -> &mut Self {
        self.groupings.sort_by_key(|g| Reverse(g.volume()));

        println!("Largest: {}", self.groupings.first().unwrap().volume());
        println!("Cross: {}", self.groupings.iter().nth(1).unwrap().volume());


        self.find_map_border();
        self.find_cross();

        self
    }


    fn find_cross(&mut self) -> &mut Self {
        let group_to_inspect = self.groupings.iter().nth(1).unwrap();

        let mut right = Position::new(0, 0);
        let mut left = Position::new(MAX, MAX);
        let mut top = Position::new(0, MAX);
        let mut bottom = Position::new(0, 0);
        
        for (pos, _) in &group_to_inspect.marks {
            if pos.y < top.y { top = pos.clone() }
            if pos.y > bottom.y { bottom = pos.clone() }

            if pos.x > right.x { right = pos.clone() }
            if pos.x < left.x { left = pos.clone() }
        }


        top.x = left.x + (right.x - left.x)/2;
        bottom.x = left.x + (right.x - left.x)/2;

        right.y = top.y + (bottom.y - top.y)/2;
        left.y = top.y + (bottom.y - top.y)/2;

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

        let group_to_inspect = self.groupings.first().unwrap();
        
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
            if pos.x > top_right.x && pos.y < height/4 { top_right = pos.clone(); }
            if pos.x < top_left.x && pos.y < height/4 { top_left = pos.clone(); }
            if pos.y < right_top.y && pos.x > width/4*3 { right_top = pos.clone(); }
            if pos.y > right_bottom.y && pos.x > width/4*3 { right_bottom= pos.clone(); }
            if pos.y > left_bottom.y && pos.x < width/4 { left_bottom = pos.clone(); }
            if pos.y < left_top.y && pos.x < width/4*3 { left_top = pos.clone(); }

            if pos.x > bottom_right.x && pos.y > height/4*3 { bottom_right = pos.clone() }
            if pos.x < bottom_left.x && pos.y > height/4*3 { bottom_left = pos.clone() }
        }


        self.draw_mark(top_right.x as u32, top_right.y as u32, 10, Color::White);
        self.draw_mark(top_left.x as u32, top_left.y as u32, 10, Color::White);

        self.draw_mark(right_top.x as u32, right_top.y as u32, 10, Color::White);
        self.draw_mark(right_bottom.x as u32, right_bottom.y as u32, 10, Color::White);

        self.draw_mark(left_bottom.x as u32, left_bottom.y as u32, 10, Color::White);
        self.draw_mark(left_top.x as u32, left_top.y as u32, 10, Color::White);

        self.draw_mark(bottom_right.x as u32, bottom_right.y as u32, 10, Color::White);
        self.draw_mark(bottom_left.x as u32, bottom_left.y as u32, 10, Color::White);

        let y = left_bottom.y - (left_bottom.y - left_top.y)/2;
        let x = left_bottom.x + (left_top.x - left_bottom.x)/2;

        println!("MIDDLE: {x}, {y}");

        self.draw_mark(x as u32, y as u32, 10, Color::White);

        self
    }


    fn draw_group(&mut self, group: &Grouping) -> &mut Self {
        for (pos, _) in &group.marks {
            let m = self.marks.get(pos).unwrap();
            self.draw_mark(pos.x as u32, pos.y as u32, 1, m.color);
        }

        self
    }
    

    fn scan_for_marks(&mut self) -> &mut Self {
        let (width, height) = self.original_img.dimensions();

        let targets = self.targets.clone();

        for x in 0..width {
            for y in 0..height {
                for BetterTarget { color, target_hex, precision } in &targets.0 {
                    let Rgb([r, g, b]) = self.original_img.get_pixel(x, y);
                    let Rgb([target_r, target_g, target_b]) = target_hex.rgb;

                    let (red_match, red_dist) = check_color(*r, target_r, *precision);
                    let (green_match, green_dist) = check_color(*g, target_g, *precision);
                    let (blue_match, blue_dist) = check_color(*b, target_b, *precision);

                    if red_match && green_match && blue_match {
                        self.marks.insert(
                            Position { x: x as i32, y: y as i32 },
                            Mark { precision: 0, color: color.clone(), x: x as i32, y: y as i32 },
                        );
                        // self.draw_mark(x, y, 1, *color);
                    }
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
                        color.rgb()
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
                    color.rgb()
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
