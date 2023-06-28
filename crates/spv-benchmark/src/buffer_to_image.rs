/// Colors a supplied buffer as an image.
/// To do that, we read all values, find min/max and use a color-ramp to
/// color the image.
///
/// Panics if the buffer is too small
pub fn safe_as_image(width: u32, height: u32, data: &[f32], output_name: &str) {
    let mut img = image::RgbImage::new(width, height);

    let (min, max) = data
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), v| {
            (min.min(*v), max.max(*v))
        });

    let colorramp = colorgrad::viridis();

    for (px, val) in img.pixels_mut().zip(data.iter()) {
        let normalized = ((val - min) / (max - min)) as f64;
        //let normalized = val.clamp(0.0, 1.0) as f64;
        let col = colorramp.at(normalized);
        px.0.copy_from_slice(&col.to_rgba8()[0..3]);
    }

    img.save(format!("{output_name}.png")).unwrap();
}
