use image::{Rgb, RgbImage, io::Reader as ImageReader};
use std::env;
use std::error::Error;

const INC: u32 = 10;

struct Mark {
    x: u32,
    y: u32,
    avg: Rgb<u8>,
    color: Color,
}

enum Color {
    White,
    Orange,
    Selected,
}

struct State {
    original_img: RgbImage,
    img: RgbImage,
    global_avg: Rgb<u8>,
}

use Direction::*;
enum Direction {
    Up,
    Down,
    Right,
    Left,
}

fn main() -> Result<(), Box<dyn Error>> {
    let now = std::time::Instant::now();
    let img_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "images/15.jpg".to_string());

    let dyn_img = ImageReader::open(&img_path)?.decode()?;
    let og_img: RgbImage = dyn_img.to_rgb8();
    let (width, height) = og_img.dimensions();

    let mut state = State {
        original_img: og_img.clone(),
        img: og_img.clone(),
        global_avg: get_avg_color_in_pixel_field_wh(&og_img, 0, 0, width, height),
    };

    let mut marks: Vec<Mark> = Vec::new();

    for x in 0..width / INC {
        for y in 0..height / INC {
            let x = x * INC;
            let y = y * INC;

            let avg = get_avg_color_in_pixel_field(&state, x, y, INC);
            let Rgb([r, g, b]) = avg;

            // tjek for hvide bolde
            if r > 200 && g > 200 && b > 200 {
                marks.push(Mark {
                    x,
                    y,
                    color: Color::White,
                    avg,
                });
            }

            // draw_rectangle(&mut img, x, y, Rgb([200, 255, 200]));
        }
    }

    for mark in &mut marks {
        mark.color = Color::Selected;
        break;
    }

    for mark in &marks {
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

    for mark in &mut marks {
        // find centrum og gå ned
        // let dist_bot = go_find_edge(&img, mark, (0, 1));
        let dist_top = go_up(&mut state, mark, Up);

        println!("dist to top: {dist_top}");
        // println!("dist to bottom: {dist_bot}");

        break;
    }

    state.img.save("out.png")?;
    println!("executed in: {}ms", now.elapsed().as_millis());
    Ok(())
}


fn color_diff(state: &State, color: Rgb<u8>) -> u32 {
    let delta_r = color.0[0] - state.global_avg.0[0];
    let delta_g = color.0[1] - state.global_avg.0[1];
    let delta_b = color.0[2] - state.global_avg.0[2];
    (delta_r as u32).pow(2) + (delta_g as u32).pow(2) + (delta_b as u32).pow(2)
}



fn go_up(state: &mut State, mark: &Mark, dir: Direction) -> i32 {
    for i in 1..3 {
        let x = match dir {
            Up => mark.x + INC / 4,
            _ => todo!(),
        };
        let y = match dir {
            Up => mark.y - INC / 2 * i,
            _ => todo!(),
        };

        let avg = get_avg_color_in_pixel_field(state, x, y, INC / 2);
        let Rgb([r, g, b]) = avg;
        println!("-> {r}, {g}, {b}");

        if i == 0 {
            continue;
        }
        draw_rectangle_with_size(state, x, y, Rgb([100, 100, 255]), INC / 2);
    }

    0
}

// fn go_find_edge(img: &mut RgbImage, mark: &Mark, dir: (i32, i32)) -> i32 {
//     let size = (INC / 2) as i32;

//     let sx = (mark.x + INC / 2) as i32;
//     let sy = (mark.y + INC / 2) as i32;

//     for i in 0..1 {
//         let x = (sx + dir.0 * i * size) as u32;
//         let y = (sy + dir.1 * i * size) as u32;

//         // compare the start avg with the new avg
//         let Rgb([r, g, b]) = mark.avg;
//         let Rgb([sr, sg, sb]) = get_avg_color_in_pixel_field(&img, x, y, size as u32);

//         let tr = (r as i32 - sr as i32).abs();
//         let tg = (g as i32 - sg as i32).abs();
//         let tb = (b as i32 - sb as i32).abs();
//         let diff = tr + tg + tb;

//         println!("diff: {}", diff);

//         draw_rectangle_with_size(&mut state, x, y, Rgb([100, 100, 255]), INC / 2);

//         // if diff < 22 xx{
//         //     return i;
//         // }
//     }

//     return 20;
// }

fn get_avg_color_in_pixel_field(state: &State, sx: u32, sy: u32, size: u32) -> Rgb<u8> {
    get_avg_color_in_pixel_field_wh(&state.original_img, sx, sy, size, size)
}
fn get_avg_color_in_pixel_field_wh(img: &RgbImage, sx: u32, sy: u32, width: u32, height: u32) -> Rgb<u8> {
    let amount = width * height;
    let mut r_sum = 0;
    let mut g_sum = 0;
    let mut b_sum = 0;

    for x in sx..sx + width {
        for y in sy..sy + height {
            let Rgb([r, g, b]) = img.get_pixel(x, y);
            r_sum += *r as u32;
            g_sum += *g as u32;
            b_sum += *b as u32;
        }
    }

    Rgb([
        (r_sum / amount) as u8,
        (g_sum / amount) as u8,
        (b_sum / amount) as u8,
    ])
}

fn draw_rectangle(state: &mut State, sx: u32, sy: u32, color: Rgb<u8>) {
    draw_rectangle_with_size(state, sx, sy, color, INC);
}

fn draw_rectangle_with_size(state: &mut State, sx: u32, sy: u32, color: Rgb<u8>, size: u32) {
    for x in sx..sx + size {
        for y in sy..sy + size {
            state.img.put_pixel(x, y, color);
        }
    }
}
