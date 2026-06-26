use image::{Rgb, RgbImage, io::Reader as ImageReader};
use std::error::Error;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Detection {
    class: String,
    confidence: f32,
    bbox: [f32; 4],
}

fn main() -> Result<(), Box<dyn Error>> {
    let detections = std::fs::read_to_string("detections.json")?;
    let detections: Vec<Detection> = serde_json::from_str(&detections)?;

    let dyn_img = ImageReader::open("image.jpg")?.decode()?;
    let mut img: RgbImage = dyn_img.to_rgb8();

    for det in &detections {
        draw_detection(&mut img, det);

    }

    img.save("output.jpg");

    Ok(())
}

fn draw_detection(img: &mut RgbImage, detection: &Detection) {
    let x = detection.bbox[0] as i32;
    let y = detection.bbox[1] as i32;
    let x2 = detection.bbox[2] as i32;
    let y2 = detection.bbox[3] as i32;

    for x in x..x2 {
        for y in y..y2 {
            // only draw the border
            if x == x || x == x2 || y == y || y == y2 {
                img.put_pixel(x as u32, y as u32, match detection.class {
                    _ => Rgb([20, 200, 20]),
                });
            }
        }
    }
}
