use image::{Rgb, RgbImage, io::Reader as ImageReader};
use std::env;
use std::error::Error;

const INC: u32 = 10;

#[derive(Debug)]
struct Mark {
    x: u32,
    y: u32,
    avg: Rgb<u8>,
    color: Color,
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
    Left,
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

    let mut c = 0;
    let target = 17;

    for mark in &mut marks {
        c += 1;
        // if c != target { continue }
        mark.color = Color::Selected;
        break;
    }

    c = 0;



    for mark in &mut marks {
        c+=1;
        // if c != target { continue }

        // find centrum og gå ned
        // let dist_bot = go_find_edge(&img, mark, (0, 1));
        let dist_top = calc_dist_to_edge(&mut state, mark, Up);
        let dist_bot = calc_dist_to_edge(&mut state, mark, Down);

        println!("{dist_top:?}, {dist_bot:?}");

        if let (Some(dist_bot), Some(dist_top)) = (dist_bot, dist_top) {
            println!("top: {dist_top}, bot: {dist_bot}");
            let new_y = dist_bot - dist_top;

            let extra_y = (new_y * (INC as i32)/4);

            if extra_y < 0 {
                mark.y -= extra_y.abs() as u32;
            }
            else {
                mark.y += extra_y as u32;
            }
        }

        let dist_right = calc_dist_to_edge(&mut state, mark, Right);
        let dist_left = calc_dist_to_edge(&mut state, mark, Left);

        if let (Some(dist_right), Some(dist_left)) = (dist_right, dist_left) {
            println!("right: {dist_right}, left: {dist_left}");
            let new_x = dist_right - dist_left;

            let extra_x = (new_x * (INC as i32)/4);
            if extra_x < 0 {
                mark.x -= extra_x.abs() as u32;
            }
            else {
                mark.x += extra_x as u32;
            }
        }

        mark.color = Color::Selected;

        // break;
    }


    for mark in &marks {
        if mark.color != Color::Selected { continue }
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

        println!("{dir:?}: delta: {delta}");

        if delta < 7500 {
            return Some(i as i32)
        }

        if i == 0 {
            continue;
        }
        draw_rectangle_with_size(state, x, y, Rgb([100, 100, 255]), INC / 2);
    }

    // panic!("Should have found an edge? {mark:#?} {dir:#?}")
    None
}


fn get_avg_color_in_pixel_field(state: &State, sx: u32, sy: u32, size: u32) -> Rgb<u8> {
    get_avg_color_in_pixel_field_wh(&state.original_img, sx, sy, size, size)
}
fn get_avg_color_in_pixel_field_wh(
    img: &RgbImage,
    sx: u32,
    sy: u32,
    width: u32,
    height: u32,
) -> Rgb<u8> {
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
