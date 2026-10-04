//! Pulls what the importer needs out of the PDF: page 1's text and the scanned image of every
//! following page.

use image::RgbImage;

use super::ImportError;

/// Reports far beyond a real game's size are rejected rather than processed.
const MAX_PAGES: usize = 101;

pub struct PdfContent {
    pub first_page_text: String,
    /// (page number, image) for pages 2.., in page order. Pages whose image can't be read are
    /// reported in `unreadable_pages` instead.
    pub page_images: Vec<(u32, RgbImage)>,
    pub unreadable_pages: Vec<(u32, String)>,
}

pub fn read(bytes: &[u8]) -> Result<PdfContent, ImportError> {
    if !bytes.starts_with(b"%PDF-") {
        return Err(ImportError::new("The file is not a PDF"));
    }

    let doc = lopdf::Document::load_mem(bytes)
        .map_err(|e| ImportError::new(format!("The PDF could not be opened: {e}")))?;
    let pages = doc.get_pages();
    if pages.len() < 2 {
        return Err(ImportError::new(
            "The PDF has no player pages (expected a summary page plus one page per player)",
        ));
    }
    if pages.len() > MAX_PAGES {
        return Err(ImportError::new(format!(
            "The PDF has {} pages; at most {MAX_PAGES} are supported",
            pages.len()
        )));
    }

    let first_page_text = pdf_extract::extract_text_from_mem_by_pages(bytes)
        .map_err(|e| ImportError::new(format!("Page 1 text could not be read: {e}")))?
        .into_iter()
        .next()
        .unwrap_or_default();

    let mut page_images = Vec::new();
    let mut unreadable_pages = Vec::new();
    for (&number, &page_id) in pages.iter().skip(1) {
        match page_image(&doc, page_id) {
            Ok(img) => page_images.push((number, img)),
            Err(reason) => unreadable_pages.push((number, reason)),
        }
    }

    Ok(PdfContent {
        first_page_text,
        page_images,
        unreadable_pages,
    })
}

/// The page's largest embedded image (the scan), decoded to RGB.
fn page_image(doc: &lopdf::Document, page_id: lopdf::ObjectId) -> Result<RgbImage, String> {
    let images = doc
        .get_page_images(page_id)
        .map_err(|e| format!("no image ({e})"))?;
    let largest = images
        .iter()
        .max_by_key(|i| i.width * i.height)
        .ok_or("no image on the page")?;

    let is_jpeg = largest
        .filters
        .as_ref()
        .is_some_and(|f| f.iter().any(|f| f == "DCTDecode"));
    if !is_jpeg {
        return Err(format!(
            "unsupported image encoding {:?}",
            largest.filters.as_deref().unwrap_or_default()
        ));
    }
    image::load_from_memory_with_format(largest.content, image::ImageFormat::Jpeg)
        .map(|i| i.into_rgb8())
        .map_err(|e| format!("image could not be decoded ({e})"))
}
