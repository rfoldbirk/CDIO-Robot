use image::{Rgb, RgbImage, io::Reader as ImageReader};
use serde::Serialize;
use std::env;
use std::error::Error;

const INC: i32 = 10;

#[derive(Debug, Serialize)]
struct Mark {
    x: i32,
    y: i32,
    #[serde(skip_serializing)]
    color: Color,
    #[serde(skip_serializing)]
    to_be_deleted: bool,
}

#[derive(Debug, PartialEq, Serialize)]
enum Color {
    White,
    Orange,
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
    Right,
    Diagonal,
    AntiDiagonal,
}

impl Direction {
    fn vec(&self) -> (i32, i32) {
        match self {
            Up => (0, -1),
            Right => (1, 0),
            Diagonal => (-1, -1),
            AntiDiagonal => (1, -1),
        }
    }
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
                    to_be_deleted: false,
                });
            }
            else if r > 190 && g > 100 && b < 140 {
                marks.push(Mark { x, y, color: Color::Orange, to_be_deleted: false });
            }
        }
    }

    for mark in &mut marks {
        let dist = edge(&mut state, mark, Up);
        let dist_l = edge(&mut state, mark, Right);

        // realign
        mark.x += (dist_l.0 - dist_l.1) * INC/4;
        mark.y += (dist.1 - dist.0) * INC/4;

        let diag = edge(&mut state, mark, Diagonal);
        let anti_diag = edge(&mut state, mark, AntiDiagonal);

        if diag.0 <= 1 || diag.1 <= 1 { mark.to_be_deleted = true }
        if anti_diag.0 <= 1 || anti_diag.1 <= 1 { mark.to_be_deleted = true }
        let dl = diag.0 + diag.1;
        let adl = anti_diag.0 + anti_diag.1;

        if dl >= adl*2 || adl >= dl*2 { mark.to_be_deleted = true }

    }

    marks.retain(|m| m.to_be_deleted == false);

    let json = serde_json::to_string_pretty(&marks).expect("rasmus har en lille diller");
    std::fs::write("./balls.json", json).expect("altså en meget lille diller");
    println!("executed in: {}ms", now.elapsed().as_millis());
    Ok(())
}


fn edge(state: &mut State, mark: &Mark, dir: Direction) -> (i32, i32) {
    let pos = (mark.x as i32, mark.y as i32);

    let inc = (INC as i32) / 2;
    let d = dir;
    let dir = d.vec();
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

        let lim = 8022;

        if da > lim && s.0 {
            r.0 += 1;
        }
        else {
            s.0 = false;
        }
        if nda > lim && s.1 {
            r.1 += 1;
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

