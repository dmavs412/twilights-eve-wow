//! Coordinate-preserving cartographic color treatment of the installed terrain bakes.
use image::{imageops, Rgba, RgbaImage};

pub fn layers(terrain: &RgbaImage, size: (u32, u32)) -> (RgbaImage, RgbaImage) {
    // Both layers use exactly the same sampling grid. Buildings, roads and river
    // bends are never redrawn or displaced to accommodate the illustration.
    let detail = imageops::resize(terrain, size.0, size.1, imageops::FilterType::Lanczos3);
    let wash = imageops::blur(&detail, 1.2);
    let mut overview = RgbaImage::new(size.0, size.1);
    for (x, y, p) in wash.enumerate_pixels() {
        let [r, g, b, _] = p.0.map(f32::from);
        let luma = (r * 0.30 + g * 0.59 + b * 0.11) / 255.0;
        let paper =
            ((x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263)) % 101) as f32 / 100.0 - 0.5;
        let mut color = if b > r * 1.12 && g > r * 1.15 {
            // Water retains its surveyed shoreline and depth changes.
            [51.0 + 92.0 * luma, 85.0 + 100.0 * luma, 91.0 + 102.0 * luma]
        } else {
            let bands = (luma * 24.0).round() / 24.0;
            let green = ((g - r) / 85.0).clamp(-0.5, 1.0);
            let rust = ((r - g) / 110.0).clamp(0.0, 1.0);
            [
                118.0 + bands * 125.0 - green * 28.0 + rust * 10.0,
                99.0 + bands * 119.0 + green * 12.0 - rust * 18.0,
                66.0 + bands * 99.0 - green * 9.0 - rust * 22.0,
            ]
        };
        // Fine local relief reads as ink, without introducing invented mountain symbols.
        let src = detail.get_pixel(x, y);
        let local = (src[0] as f32 * 0.30 + src[1] as f32 * 0.59 + src[2] as f32 * 0.11) / 255.0;
        let ink = ((luma - local) * 0.65).clamp(0.0, 0.18);
        for (channel, c) in color.iter_mut().enumerate() {
            // Retain the source's value contrast so streets and roads remain legible
            // even before the close terrain layer takes over.
            *c = ((*c * 0.5 + src[channel] as f32 * 0.5) * (1.0 - ink) + paper * 3.0)
                .clamp(0.0, 255.0);
        }
        overview.put_pixel(
            x,
            y,
            Rgba([color[0] as u8, color[1] as u8, color[2] as u8, 255]),
        );
    }
    (overview, detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_preserves_survey_buildings_and_roads_on_the_same_grid() {
        let mut source = RgbaImage::from_pixel(41, 29, Rgba([60, 100, 35, 255]));
        source.put_pixel(17, 23, Rgba([230, 230, 230, 255]));
        let (overview, detail) = layers(&source, source.dimensions());
        assert_eq!(detail, source);
        assert_eq!(overview.dimensions(), source.dimensions());
        assert_eq!(detail.get_pixel(17, 23), source.get_pixel(17, 23));
    }
}
