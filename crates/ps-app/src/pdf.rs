//! Render a self-contained HTML document to PDF with an offscreen WKWebView.

/// Shown when this operating system has no PDF renderer.
#[cfg(not(target_os = "macos"))]
const UNSUPPORTED: &str = "PDF export is available on macOS.";

/// Turns a complete HTML document into PDF bytes.
#[cfg(not(target_os = "macos"))]
pub(crate) fn html_to_pdf(_html: &str) -> Result<Vec<u8>, String> {
    Err(UNSUPPORTED.to_owned())
}

#[cfg(target_os = "macos")]
use std::sync::mpsc;
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
use block2::RcBlock;
#[cfg(target_os = "macos")]
use objc2::MainThreadMarker;
#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2_core_foundation::{
    CFRunLoop, CFTimeInterval, CGPoint, CGRect, CGSize, kCFRunLoopDefaultMode,
};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSData, NSError, NSString};
#[cfg(target_os = "macos")]
use objc2_web_kit::{WKPDFConfiguration, WKWebView, WKWebViewConfiguration};

#[cfg(target_os = "macos")]
const LOAD_TIMEOUT: Duration = Duration::from_secs(15);
#[cfg(target_os = "macos")]
const PDF_TIMEOUT: Duration = Duration::from_secs(20);

/// Turns a complete HTML document into PDF bytes.
#[cfg(target_os = "macos")]
pub(crate) fn html_to_pdf(html: &str) -> Result<Vec<u8>, String> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| "PDF export must run on the main thread.".to_string())?;
    html_to_pdf_on_main(mtm, html)
}

#[cfg(target_os = "macos")]
fn html_to_pdf_on_main(mtm: MainThreadMarker, html: &str) -> Result<Vec<u8>, String> {
    // SAFETY: `MainThreadMarker` proves we are on the AppKit main thread.
    // The WKWebView stays retained until the PDF callback fires.
    let webview: Retained<WKWebView> = unsafe {
        let config = WKWebViewConfiguration::new(mtm);
        let frame = CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(816.0, 1056.0));
        let webview = WKWebView::initWithFrame_configuration(mtm.alloc(), frame, &config);
        let ns_html = NSString::from_str(html);
        webview.loadHTMLString_baseURL(&ns_html, None);
        webview
    };

    let deadline = Instant::now() + LOAD_TIMEOUT;
    while unsafe { webview.isLoading() } && Instant::now() < deadline {
        pump_run_loop(0.05);
    }
    if unsafe { webview.isLoading() } {
        return Err("The document took too long to lay out for PDF.".to_string());
    }
    for _ in 0..6 {
        pump_run_loop(0.05);
    }

    let (tx, rx) = mpsc::channel();
    let block = RcBlock::new(move |data: *mut NSData, error: *mut NSError| {
        if !error.is_null() {
            let message = unsafe { &*error }.localizedDescription().to_string();
            let _ = tx.send(Err(message));
            return;
        }
        if data.is_null() {
            let _ = tx.send(Err("WebKit returned an empty PDF.".to_string()));
            return;
        }
        let bytes = unsafe { &*data }.to_vec();
        let _ = tx.send(Ok(bytes));
    });
    unsafe {
        let pdf_config = WKPDFConfiguration::new(mtm);
        webview.createPDFWithConfiguration_completionHandler(Some(&pdf_config), &block);
    }

    let deadline = Instant::now() + PDF_TIMEOUT;
    loop {
        match rx.try_recv() {
            Ok(result) => {
                let bytes = result?;
                if !bytes.starts_with(b"%PDF") {
                    return Err("WebKit did not produce a PDF file.".to_string());
                }
                return Ok(bytes);
            }
            Err(mpsc::TryRecvError::Empty) => {
                if Instant::now() >= deadline {
                    return Err("PDF export timed out.".to_string());
                }
                pump_run_loop(0.05);
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err("PDF export was interrupted.".to_string());
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn pump_run_loop(seconds: f64) {
    // SAFETY: pumping the current thread's run loop from the main thread.
    let _ = unsafe {
        CFRunLoop::run_in_mode(kCFRunLoopDefaultMode, CFTimeInterval::from(seconds), true)
    };
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn waits_more_than_a_second_for_layout() {
        assert!(super::LOAD_TIMEOUT > std::time::Duration::from_secs(1));
    }
}

#[cfg(all(test, not(target_os = "macos")))]
mod host_tests {
    #[test]
    fn unsupported_message_names_macos() {
        assert!(super::UNSUPPORTED.contains("macOS"));
    }
}
