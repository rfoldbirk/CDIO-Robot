use image::{Rgb, RgbImage, io::Reader as ImageReader};
use std::env;
use std::error::Error;

const INC: i32 = 10;

#[derive(Debug)]
struct Mark {
    x: i32,
    y: i32,
    avg: Rgb<u8>,
    color: Color,
    tbd: bool, // to be deleted;
}

#[derive(Debug, PartialEq)]
enum Color {
    White,
    Orange,
    Selected,
}

#[derive(Debug)]
struct State {
    original_img: RgbImage,
    img: RgbImage,
    global_avg: Rgb<u8>,
}

use Direction::*;
#[derive(Debug)]
enum Direction {
    Up,
    Down,
    Right,
    Diagonal,
    AntiDiagonal,
    Left,
}

impl Direction {
    fn vec(&self) -> (i32, i32) {
        match self {
            Up => (0, -1),
            // Down => (0, 1),
            Right => (1, 0),
            Diagonal => (-1, -1),
            AntiDiagonal => (1, -1),
            // Left => (-1, 0),
            _ => panic!("lad vær")
        }
    }

    // fn offset(&self) -> (i32, i32) {
    //     match self {
    //         Up => (INC/4, 1),
    //         Right => (0, INC/4),
    //         Diagonal => (1, 1),
    //         _ => panic!("lad vær")
    //     }
    // }
}

fn main() -> Result<(), Box<dyn Error>> {
    let now = std::time::Instant::now();
    let img_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "images/12.jpg".to_string());

    let dyn_img = ImageReader::open(&img_path)?.decode()?;
    let og_img: RgbImage = dyn_img.to_rgb8();
    let (width, height) = og_img.dimensions();


    let mut state = State {
        original_img: og_img.clone(),
        img: og_img.clone(),
        global_avg: get_avg_color_in_pixel_field_wh(&og_img, 0, 0, width as i32, height as i32),
    };

    let mut marks: Vec<Mark> = Vec::new();

    // mark everything that might be a ball!
    for x in 0..width as i32 / INC {
        for y in 0..height as i32 / INC {
            let x = x * INC;
            let y = y * INC;

            let avg = get_avg_color_in_pixel_field(&state, x, y, INC);
            let Rgb([r, g, b]) = avg;

            let delta = color_diff(&state, avg);
            if delta < 8000 { continue }
            // tjek for hvide bolde
            if r > 200 && g > 200 && b > 200 {
                marks.push(Mark {
                    x,
                    y,
                    color: Color::White,
                    avg,
                    tbd: false,
                });
            }
            else if r > 190 && g > 100 && b < 140 {
                marks.push(Mark { x, y, avg, color: Color::Orange, tbd: false });
            }

            // draw_rectangle(&mut img, x, y, Rgb([200, 255, 200]));
        }
    }


    for mark in &mut marks {
        // break;
        // let dist = edge(&mut state, mark, Up);
        // println!("dist: {} {}", dist.0, dist.1);

        // break;
        // //
        // //
        // // find centrum og gå ned
        // // let dist_bot = go_find_edge(&img, mark, (0, 1));
        // let dist_top = calc_dist_to_edge(&mut state, mark, Up);
        // let dist_bot = calc_dist_to_edge(&mut state, mark, Down);

        // println!("{dist_top:?}, {dist_bot:?}");


        // if let (Some(dist_bot), Some(dist_top)) = (dist_bot, dist_top) {
        //     // println!("top: {dist_top}, bot: {dist_bot}");
        //     let new_y = dist_bot - dist_top;

        //     let extra_y = new_y * (INC as i32)/4;

        //     if extra_y < 0 {
        //         mark.y -= extra_y.abs() as u32;
        //     }
        //     else {
        //         mark.y += extra_y as u32;
        //     }
        // }
        // else {
        //     mark.tbd = true;
        // }

        // let dist_right = calc_dist_to_edge(&mut state, mark, Right);
        // let dist_left = calc_dist_to_edge(&mut state, mark, Left);

        // if let (Some(dist_right), Some(dist_left)) = (dist_right, dist_left) {
        //     // println!("right: {dist_right}, left: {dist_left}");
        //     let new_x = dist_right - dist_left;

        //     let extra_x = new_x * (INC as i32)/4;
        //     if extra_x < 0 {
        //         mark.x -= extra_x.abs() as u32;
        //     }
        //     else {
        //         mark.x += extra_x as u32;
        //     }
        // }
        // else {
        //     mark.tbd = true;
        // }

        // mark.color = Color::Selected;

        // break;
    }


    let mut i = 0;
    let target = 13;

    for mark in &mut marks {
        i += 1;
        // if i != target { mark.tbd = true; continue; }
        // break;
        let dist = edge(&mut state, mark, Up, true);
        let dist_l = edge(&mut state, mark, Right, true);

        println!("up: {}-{}", dist.0, dist.1);
        println!("right: {}-{}", dist_l.0, dist_l.1);


        // realign
        mark.x += (dist_l.0 - dist_l.1) * INC/4;
        mark.y += (dist.1 - dist.0) * INC/4;

        let diag = edge(&mut state, mark, Diagonal, true);
        let anti_diag = edge(&mut state, mark, AntiDiagonal, true);

        println!("diag: {} {}", diag.0, diag.1);
        println!("anti: {} {}", anti_diag.0, anti_diag.1);
        if diag.0 <= 1 || diag.1 <= 1 { mark.tbd = true }
        if anti_diag.0 <= 1 || anti_diag.1 <= 1 { mark.tbd = true }
        let dl = diag.0 + diag.1;
        let adl = anti_diag.0 + anti_diag.1;

        if dl >= adl*2 || adl >= dl*2 { mark.tbd = true }

        // if diag.0 + diag.1 > 5 { mark.tbd = true }
        // println!("diag: {} {}", diag.0, diag.1);
        // println!("anti_diag: {} {}", anti_diag.0, anti_diag.1);


        // println!("dist: {} {}", dist.0, dist.1);

        // break;
    }

    for mark in &marks {
        if mark.tbd { continue }
        // if mark.color != Color::Selected { continue }
        // tegn
        draw_rectangle(
            &mut state,
            mark.x,
            mark.y,
            match mark.color {
                Color::White => Rgb([200, 255, 200]),
                Color::Orange => Rgb([255, 165, 0]),
                Color::Selected => Rgb([255, 220, 220]),
            },
        );

        // break;
    }



    state.img.save("out.png")?;
    println!("executed in: {}ms", now.elapsed().as_millis());
    Ok(())
}


fn edge(state: &mut State, mark: &Mark, dir: Direction, draw: bool) -> (i32, i32) {
    let pos = (mark.x as i32, mark.y as i32);

    let inc = (INC as i32) / 2;
    let dir = dir.vec();
    let dir = (dir.0 * inc, dir.1 * inc);

    let mut r = (0, 0);
    let mut s = (true, true);

    for i in 0..4 {
        let new_pos = (INC/4+ pos.0 + i * dir.0, INC/4+pos.1 + i * dir.1);
        let neg_pos = (INC/4+ pos.0 + -i * dir.0, INC/4+pos.1 + -i * dir.1);

        let avg = get_avg_color_in_pixel_field(state, new_pos.0, new_pos.1, INC / 2);
        let da = color_diff(state, avg);

        let n_avg = get_avg_color_in_pixel_field(state, neg_pos.0, neg_pos.1, INC / 2);
        let nda = color_diff(state, n_avg);

        if draw { println!("up: {da}, down: {nda}") }

        let lim = 4458;

        if da > lim && s.0 {
            r.0 += 1;
            if draw {
                draw_rectangle_with_size(state, new_pos.0, new_pos.1, Rgb([200, 150, 105]), INC / 2);
            }
        }
        else {
            s.0 = false;
        }
        if nda > lim && s.1 {
            r.1 += 1;
            if draw {
                draw_rectangle_with_size(state, neg_pos.0, neg_pos.1, Rgb([150, 150, 105]), INC / 2);
            }
        }
        else {
            s.1 = false;
        }
    }

    r
}

fn color_diff(state: &State, color: Rgb<u8>) -> u32 {
    let delta_r = color.0[0].abs_diff(state.global_avg.0[0]);
    let delta_g = color.0[1].abs_diff(state.global_avg.0[1]);
    let delta_b = color.0[2].abs_diff(state.global_avg.0[2]);
    (delta_r as u32).pow(2) + (delta_g as u32).pow(2) + (delta_b as u32).pow(2)
}

fn calc_dist_to_edge(state: &mut State, mark: &Mark, dir: Direction) -> Option<i32> {
    let res = 2;

    for i in 0..10 {
        let x = match dir {
            Right => mark.x + INC/res * i,
            Left => mark.x - INC/res * i + INC/res,
            _ => mark.x + INC / (res*2),
        };
        let y = match dir {
            Up => mark.y - INC / res * i + INC/res,
            Down => mark.y + INC / res * i,
            _ => mark.y + INC/(res*2),
        };

        let avg = get_avg_color_in_pixel_field(state, x, y, INC / res);
        let delta = color_diff(state, avg);

        // println!("{dir:?}: delta: {delta}");

        if delta < 7500 {
            if i == 0 {
                return None;
            }
            return Some(i as i32)
        }

        if i == 0 {
            continue;
        }
        // draw_rectangle_with_size(state, x, y, Rgb([100, 100, 255]), INC / 2);
    }

    // panic!("Should have found an edge? {mark:#?} {dir:#?}")
    None
}


fn get_avg_color_in_pixel_field(state: &State, sx: i32, sy: i32, size: i32) -> Rgb<u8> {
    get_avg_color_in_pixel_field_wh(&state.original_img, sx, sy, size, size)
}
fn get_avg_color_in_pixel_field_wh(
    img: &RgbImage,
    sx: i32,
    sy: i32,
    width: i32,
    height: i32,
) -> Rgb<u8> {
    let amount = width * height;
    let mut r_sum = 0;
    let mut g_sum = 0;
    let mut b_sum = 0;

    let w = if sx+width >= img.dimensions().0 as i32 {
        img.dimensions().0 as i32 -2
    } else {
        sx+width
    };

    let h = if sy+height >= img.dimensions().1 as i32 {
        img.dimensions().1 as i32 -2
    } else {
        sy+height
    };

    for x in sx..w {
        for y in sy..h {
            let Rgb([r, g, b]) = img.get_pixel(x as u32, y as u32);
            r_sum += *r as u32;
            g_sum += *g as u32;
            b_sum += *b as u32;
        }
    }

    let amount = amount as u32;

    Rgb([
        (r_sum / amount) as u8,
        (g_sum / amount) as u8,
        (b_sum / amount) as u8,
    ])
}

fn draw_rectangle(state: &mut State, sx: i32, sy: i32, color: Rgb<u8>) {
    draw_rectangle_with_size(state, sx, sy, color, INC);
}

fn draw_rectangle_with_size(state: &mut State, sx: i32, sy: i32, color: Rgb<u8>, size: i32) {
    for x in sx..sx + size {
        for y in sy..sy + size {
            state.img.put_pixel(x as u32, y as u32, color);
        }
    }
}
