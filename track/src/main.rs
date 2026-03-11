use std::error::Error;
use std::env;
use image::{io::Reader as ImageReader, Rgb, RgbImage};

static INC: u32 = 10;

fn main() -> Result<(), Box<dyn Error>> {
    let now = std::time::Instant::now();
    let img_path = env::args().nth(4).unwrap_or_else(|| "img.jpg".to_string());
    let R: u8 = env::args().nth(1).unwrap().parse().unwrap();
    let G: u8 = env::args().nth(2).unwrap().parse().unwrap();
    let B: u8 = env::args().nth(3).unwrap().parse().unwrap();

    let dyn_img = ImageReader::open(&img_path)?.decode()?;
    let img: RgbImage = dyn_img.to_rgb8();
    let (width, height) = img.dimensions();


    let mut out = img.clone();
    // draw_rectangle(&mut out, 0, 0);

    let Rgb([r, g, b]) = get_avg_color_in_pixel_field(&img, 0, 0);
    // println!("avg: {r}, {g}, {b}");

    for _y in 0..height/INC {
        for _x in 0..width/INC {
            let x = _x * INC;
            let y = _y * INC;

            let Rgb([r, g, b]) = get_avg_color_in_pixel_field(&img, x, y);

            // 150 150 110
            if r > R && g > G && b < B {

                println!("avg: {r}, {g}, {b}");

                draw_rectangle(&mut out, x, y);
            }
       }
    }

    out.save("out.png")?;
    println!("Wrote out.png");

    println!("->: {}", now.elapsed().as_millis());

    Ok(())
}


fn get_avg_color_in_pixel_field(img: &RgbImage, sx: u32, sy: u32) -> Rgb<u8> {
    let mut r_sum = 0;
    let mut g_sum = 0;
    let mut b_sum = 0;

    for y in sy..sy + INC {
        for x in sx..sx + INC {
            let Rgb([r, g, b]) = img.get_pixel(x, y);
            r_sum += *r as u32;
            g_sum += *g as u32;
            b_sum += *b as u32;
        }
    }

    let count = INC * INC;
    Rgb([(r_sum / count) as u8, (g_sum / count) as u8, (b_sum / count) as u8])

}


fn draw_rectangle(img: &mut RgbImage, sx: u32, sy: u32) {
    let color = Rgb([255, 112, 26]);

    for y in sy..sy + INC {
        for x in sx..sx + INC {
            img.put_pixel(x, y, color);
        }
    }
}
