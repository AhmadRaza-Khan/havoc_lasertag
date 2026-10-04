//! OCR for reading text out of images at known positions (fixed-layout documents).
//!
//! Uses `ocrs` (pure Rust) with its models embedded in the binary (see `build.rs`).
//!
//! The text detector takes a fixed 600×800 input: small images are padded, large ones shrunk.
//! A full page is shrunk ~3.7× and loses accuracy, while a single field crop is read at native
//! resolution but costs a whole detector pass. So the field crops of a page are packed, at
//! native scale, into 600×800 canvases: one pass per canvas reads many fields accurately.

use std::sync::OnceLock;

use image::{GrayImage, Rgb, RgbImage, imageops};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use rten_imageproc::RotatedRect;

static DETECTION_MODEL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/text-detection.onnx"));
static RECOGNITION_MODEL: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/text-recognition.onnx"));

/// Detector input size (width × height).
const CANVAS_W: u32 = 600;
const CANVAS_H: u32 = 800;
/// White margin around each crop so the detector sees whole words.
const PAD: u32 = 30;
/// Pixels darker than this count as text ("ink"); light grey row stripes are ~243.
const INK_LUMA: u8 = 215;
/// Fewer ink pixels than this means the field is blank.
const MIN_INK: usize = 15;

/// A rectangle in image pixels (x0/y0 inclusive, x1/y1 exclusive).
#[derive(Debug, Clone, Copy)]
pub struct Region {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

pub struct Ocr {
    engine: OcrEngine,
}

static OCR: OnceLock<Result<Ocr, String>> = OnceLock::new();

impl Ocr {
    /// The shared engine, loading the models on first use (~10 ms).
    pub fn global() -> anyhow::Result<&'static Ocr> {
        OCR.get_or_init(|| Ocr::load().map_err(|e| e.to_string()))
            .as_ref()
            .map_err(|e| anyhow::anyhow!("OCR models failed to load: {e}"))
    }

    fn load() -> anyhow::Result<Ocr> {
        let engine = OcrEngine::new(OcrEngineParams {
            detection_model: Some(Model::load_static_slice(DETECTION_MODEL)?),
            recognition_model: Some(Model::load_static_slice(RECOGNITION_MODEL)?),
            ..Default::default()
        })?;
        Ok(Ocr { engine })
    }

    /// Reads the text of each region. `None` for regions with no ink (blank fields).
    /// Multiple lines within a region are joined with a space.
    pub fn read_regions(
        &self,
        image: &RgbImage,
        regions: &[Region],
    ) -> anyhow::Result<Vec<Option<String>>> {
        let gray = imageops::grayscale(image);
        let mut results: Vec<Option<String>> = vec![None; regions.len()];

        for canvas in pack(image, &gray, regions) {
            let input = self.engine.prepare_input(ImageSource::from_bytes(
                canvas.image.as_raw(),
                canvas.image.dimensions(),
            )?)?;
            let words = self.engine.detect_words(&input)?;

            let mut lines: Vec<Vec<RotatedRect>> = Vec::new();
            let mut owners: Vec<usize> = Vec::new();
            for cell in &canvas.cells {
                let cell_words: Vec<RotatedRect> = words
                    .iter()
                    .filter(|w| cell.contains(w.center().x, w.center().y))
                    .copied()
                    .collect();
                for line in self.engine.find_text_lines(&input, &cell_words) {
                    lines.push(line);
                    owners.push(cell.region);
                }
            }

            for (region, text) in owners
                .into_iter()
                .zip(self.engine.recognize_text(&input, &lines)?)
            {
                let Some(text) = text else { continue };
                let text = text.to_string();
                let text = text.trim();
                if text.is_empty() {
                    continue;
                }
                match &mut results[region] {
                    Some(existing) => {
                        existing.push(' ');
                        existing.push_str(text);
                    }
                    slot => *slot = Some(text.to_string()),
                }
            }
        }
        Ok(results)
    }
}

struct Cell {
    region: usize,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl Cell {
    fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x as f32
            && x < (self.x + self.w) as f32
            && y >= self.y as f32
            && y < (self.y + self.h) as f32
    }
}

struct Canvas {
    image: RgbImage,
    cells: Vec<Cell>,
}

/// Shelf-packs the non-blank region crops into detector-sized canvases. A crop too wide to
/// share gets its own canvas with the detector's aspect ratio (scaled down slightly by it).
fn pack(image: &RgbImage, gray: &GrayImage, regions: &[Region]) -> Vec<Canvas> {
    let blank = || RgbImage::from_pixel(CANVAS_W, CANVAS_H, Rgb([255, 255, 255]));
    let mut canvases: Vec<Canvas> = Vec::new();
    let (mut cx, mut cy, mut shelf_h) = (0u32, CANVAS_H, 0u32);

    for (index, r) in regions.iter().enumerate() {
        let r = clamp(*r, image.width(), image.height());
        let (w, h) = (r.x1 - r.x0, r.y1 - r.y0);
        if w == 0 || h == 0 || ink(gray, r) < MIN_INK {
            continue;
        }
        let crop = imageops::crop_imm(image, r.x0, r.y0, w, h).to_image();
        let (bw, bh) = (w + 2 * PAD, h + 2 * PAD);

        if bw > CANVAS_W || bh > CANVAS_H {
            let ch = (bw * CANVAS_H).div_ceil(CANVAS_W).max(bh);
            let cw = (ch * CANVAS_W).div_ceil(CANVAS_H).max(bw);
            let mut own = RgbImage::from_pixel(cw, ch, Rgb([255, 255, 255]));
            imageops::overlay(&mut own, &crop, PAD as i64, PAD as i64);
            canvases.push(Canvas {
                image: own,
                cells: vec![Cell {
                    region: index,
                    x: 0,
                    y: 0,
                    w: cw,
                    h: ch,
                }],
            });
            cy = CANVAS_H; // next shared crop starts a fresh canvas
            continue;
        }

        if cx + bw > CANVAS_W {
            cx = 0;
            cy += shelf_h;
            shelf_h = 0;
        }
        if cy + bh > CANVAS_H {
            canvases.push(Canvas {
                image: blank(),
                cells: Vec::new(),
            });
            cx = 0;
            cy = 0;
            shelf_h = 0;
        }
        let canvas = canvases.last_mut().expect("a canvas was just pushed");
        imageops::overlay(
            &mut canvas.image,
            &crop,
            (cx + PAD) as i64,
            (cy + PAD) as i64,
        );
        canvas.cells.push(Cell {
            region: index,
            x: cx,
            y: cy,
            w: bw,
            h: bh,
        });
        cx += bw;
        shelf_h = shelf_h.max(bh);
    }
    canvases
}

fn clamp(r: Region, width: u32, height: u32) -> Region {
    let x1 = r.x1.min(width);
    let y1 = r.y1.min(height);
    Region {
        x0: r.x0.min(x1),
        y0: r.y0.min(y1),
        x1,
        y1,
    }
}

fn ink(gray: &GrayImage, r: Region) -> usize {
    (r.y0..r.y1)
        .flat_map(|y| (r.x0..r.x1).map(move |x| (x, y)))
        .filter(|&(x, y)| gray.get_pixel(x, y)[0] < INK_LUMA)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(x0: u32, y0: u32, x1: u32, y1: u32) -> Region {
        Region { x0, y0, x1, y1 }
    }

    #[test]
    fn packs_small_crops_together_and_skips_blank_ones() {
        let mut img = RgbImage::from_pixel(1000, 1000, Rgb([255, 255, 255]));
        // ink in the first three regions only
        for (x, y) in [(10, 10), (210, 10), (410, 10)] {
            for dx in 0..20 {
                for dy in 0..5 {
                    img.put_pixel(x + dx, y + dy, Rgb([0, 0, 0]));
                }
            }
        }
        let gray = imageops::grayscale(&img);
        let regions = [
            region(0, 0, 100, 50),
            region(200, 0, 300, 50),
            region(400, 0, 500, 50),
            region(600, 0, 700, 50), // blank
        ];
        let canvases = pack(&img, &gray, &regions);
        assert_eq!(canvases.len(), 1);
        let used: Vec<usize> = canvases[0].cells.iter().map(|c| c.region).collect();
        assert_eq!(used, [0, 1, 2]);
    }

    #[test]
    fn oversized_crop_gets_its_own_canvas() {
        let mut img = RgbImage::from_pixel(1200, 200, Rgb([255, 255, 255]));
        for x in 0..1100 {
            img.put_pixel(x, 50, Rgb([0, 0, 0]));
        }
        let gray = imageops::grayscale(&img);
        let canvases = pack(&img, &gray, &[region(0, 0, 1150, 100)]);
        assert_eq!(canvases.len(), 1);
        let c = &canvases[0];
        assert!(c.image.width() >= 1150 + 2 * PAD);
        // keeps the detector's 3:4 aspect ratio
        let ratio = c.image.width() as f32 / c.image.height() as f32;
        assert!((ratio - 0.75).abs() < 0.01, "ratio {ratio}");
    }
}
