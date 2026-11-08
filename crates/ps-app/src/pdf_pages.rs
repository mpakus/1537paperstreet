//! Draw PDF pages into PNG images with PDFKit and Core Graphics.
//!
//! The reading pane already shows images. Page bitmaps use that path, so a PDF
//! does not depend on the webview's PDF plugin.

use std::io::Cursor;
use std::path::Path;
use std::sync::Mutex;

use objc2::AnyThread;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGContext, CGImageAlphaInfo, CGPDFBox, CGPDFPage,
};
use objc2_foundation::{NSString, NSURL};
use objc2_pdf_kit::PDFDocument;

const MAX_PAGES: usize = 40;
const MAX_EDGE_PX: f64 = 1400.0;
const MAX_PAGE_BYTES: usize = 32 * 1024 * 1024;

struct RenderedPages {
    pngs: Vec<Vec<u8>>,
    total: usize,
}

/// HTML for a PDF on disk. `None` when the file is not a readable PDF.
pub(crate) fn preview_html(path: &Path) -> Option<String> {
    let rendered = render_pages(path)?;
    if rendered.pngs.is_empty() {
        return None;
    }
    let sources: Vec<String> = rendered
        .pngs
        .iter()
        .map(|png| format!("data:image/png;base64,{}", base64(png)))
        .collect();
    Some(ps_render::render_pdf_page_images(
        &sources,
        sources.len(),
        rendered.total,
    ))
}

fn render_pages(path: &Path) -> Option<RenderedPages> {
    let _guard = pdf_lock();
    let text = path.to_str()?;
    let url = NSURL::fileURLWithPath(&NSString::from_str(text));
    // SAFETY: `url` is a file URL for this path. `initWithURL` is the designated
    // initializer and returns `None` when the file is not a PDF.
    let document = unsafe { PDFDocument::initWithURL(PDFDocument::alloc(), &url) }?;
    // SAFETY: `pageCount` reads the open document and does not retain extra state.
    let total = unsafe { document.pageCount() };
    if total == 0 {
        return None;
    }
    let shown = total.min(MAX_PAGES);
    let mut pngs = Vec::with_capacity(shown);
    for index in 0..shown {
        // SAFETY: `index` is below `pageCount`.
        let Some(page) = (unsafe { document.pageAtIndex(index) }) else {
            continue;
        };
        // SAFETY: `page` stays alive for this iteration. The returned page is retained.
        let Some(cg_page) = (unsafe { page.pageRef() }) else {
            continue;
        };
        if let Some(png) = render_page(&cg_page) {
            pngs.push(png);
        }
    }
    if pngs.is_empty() {
        return None;
    }
    Some(RenderedPages { pngs, total })
}

fn render_page(page: &CGPDFPage) -> Option<Vec<u8>> {
    let bounds = CGPDFPage::box_rect(Some(page), CGPDFBox::CropBox);
    let mut width_pt = bounds.size.width;
    let mut height_pt = bounds.size.height;
    if !width_pt.is_finite() || !height_pt.is_finite() || width_pt < 1.0 || height_pt < 1.0 {
        let media = CGPDFPage::box_rect(Some(page), CGPDFBox::MediaBox);
        width_pt = media.size.width;
        height_pt = media.size.height;
    }
    if !width_pt.is_finite() || !height_pt.is_finite() || width_pt < 1.0 || height_pt < 1.0 {
        return None;
    }
    let scale = (MAX_EDGE_PX / width_pt.max(height_pt)).clamp(0.25, 2.0);
    let width = (width_pt * scale).round();
    let height = (height_pt * scale).round();
    if !width.is_finite() || !height.is_finite() || width < 1.0 || height < 1.0 {
        return None;
    }
    let width = usize::try_from(width as u64).ok()?;
    let height = usize::try_from(height as u64).ok()?;
    let row = width.checked_mul(4)?;
    let len = row.checked_mul(height)?;
    if len > MAX_PAGE_BYTES {
        return None;
    }
    let mut pixels = vec![0_u8; len];
    let space = CGColorSpace::new_device_rgb()?;
    // SAFETY: `pixels` is `width * height * 4` bytes with stride `row`. The bitmap
    // info is 8-bit RGBA, matching device RGB plus premultiplied-last alpha.
    // Core Graphics does not free a caller-owned buffer.
    let context = unsafe {
        CGBitmapContextCreate(
            pixels.as_mut_ptr().cast(),
            width,
            height,
            8,
            row,
            Some(&space),
            CGImageAlphaInfo::PremultipliedLast.0,
        )?
    };
    let rect = CGRect::new(
        CGPoint::new(0.0, 0.0),
        CGSize::new(width as f64, height as f64),
    );
    CGContext::set_rgb_fill_color(Some(&context), 1.0, 1.0, 1.0, 1.0);
    CGContext::fill_rect(Some(&context), rect);
    let transform = CGPDFPage::drawing_transform(Some(page), CGPDFBox::CropBox, rect, 0, true);
    CGContext::concat_ctm(Some(&context), transform);
    CGContext::draw_pdf_page(Some(&context), Some(page));
    drop(context);
    encode_png(width, height, &pixels)
}

fn encode_png(width: usize, height: usize, rgba: &[u8]) -> Option<Vec<u8>> {
    let width = u32::try_from(width).ok()?;
    let height = u32::try_from(height).ok()?;
    let mut output = Cursor::new(Vec::new());
    let mut encoder = png::Encoder::new(&mut output, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(rgba).ok()?;
    drop(writer);
    Some(output.into_inner())
}

fn pdf_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_the_sample_pdf_page() {
        let pngs = render_pages(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/hello.pdf"
        )))
        .expect("pdf")
        .pngs;
        assert_eq!(pngs.len(), 1);
        assert!(pngs[0].starts_with(b"\x89PNG\r\n\x1a\n"));
    }
}
