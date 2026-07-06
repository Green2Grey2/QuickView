use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::Rc,
};

use gtk::prelude::*;
use gtk4 as gtk;

use quickview_core::{
    cache, fs,
    ocr::{tesseract, tesseract::OcrOptions, tsv},
};

use crate::widgets::image_overlay::ImageOverlayWidget;

/// Basic metadata about the currently displayed file, for the info display.
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// File name without the directory.
    pub name: String,
    /// Texture width in pixels (0 when `load_failed`).
    pub width: i32,
    /// Texture height in pixels (0 when `load_failed`).
    pub height: i32,
    /// File size in bytes; `None` if metadata could not be read.
    pub size_bytes: Option<u64>,
    /// True when the image could not be decoded.
    pub load_failed: bool,
}

type FileLoadedCallback = Box<dyn Fn(&FileInfo)>;

#[derive(Clone)]
pub struct ViewerController {
    overlay: ImageOverlayWidget,

    current_file: Rc<RefCell<PathBuf>>,
    dir_images: Rc<RefCell<Vec<PathBuf>>>,
    dir_index: Rc<Cell<usize>>,

    ocr: Rc<RefCell<OcrOptions>>,

    // Monotonic ids to ignore late results from superseded jobs.
    decode_job_id: Rc<Cell<u64>>,
    ocr_job_id: Rc<Cell<u64>>,

    on_file_loaded: Rc<RefCell<Option<FileLoadedCallback>>>,
    last_file_info: Rc<RefCell<Option<FileInfo>>>,
}

impl ViewerController {
    pub fn new(initial_file: PathBuf, ocr: OcrOptions) -> Self {
        let overlay = ImageOverlayWidget::new();

        let current_file = Rc::new(RefCell::new(initial_file.clone()));
        let (dir_images, dir_index) = compute_dir_index(&initial_file);

        let this = Self {
            overlay,
            current_file,
            dir_images: Rc::new(RefCell::new(dir_images)),
            dir_index: Rc::new(Cell::new(dir_index)),
            ocr: Rc::new(RefCell::new(ocr)),
            decode_job_id: Rc::new(Cell::new(0)),
            ocr_job_id: Rc::new(Cell::new(0)),
            on_file_loaded: Rc::new(RefCell::new(None)),
            last_file_info: Rc::new(RefCell::new(None)),
        };

        this.load_file(&initial_file);
        this
    }

    pub fn widget(&self) -> gtk::Widget {
        self.overlay.widget()
    }

    pub fn overlay(&self) -> ImageOverlayWidget {
        self.overlay.clone()
    }

    pub fn current_file(&self) -> PathBuf {
        self.current_file.borrow().clone()
    }

    /// Change the OCR settings for subsequent loads.
    ///
    /// Takes effect on the next `load_file` (jobs already in flight keep the
    /// settings they started with).
    pub fn set_ocr_options(&self, ocr: OcrOptions) {
        *self.ocr.borrow_mut() = ocr;
    }

    /// Register a callback fired whenever a file finishes loading (or fails).
    ///
    /// If a file has already been loaded, the callback is invoked immediately
    /// with its info so late registration (e.g. after `new()`) misses nothing.
    pub fn connect_file_loaded(&self, f: impl Fn(&FileInfo) + 'static) {
        if let Some(info) = self.last_file_info.borrow().as_ref() {
            f(info);
        }
        *self.on_file_loaded.borrow_mut() = Some(Box::new(f));
    }

    fn emit_file_loaded(&self, info: FileInfo) {
        if let Some(cb) = self.on_file_loaded.borrow().as_ref() {
            cb(&info);
        }
        *self.last_file_info.borrow_mut() = Some(info);
    }

    pub fn load_file(&self, path: &Path) {
        *self.current_file.borrow_mut() = path.to_path_buf();

        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let size_bytes = std::fs::metadata(path).ok().map(|m| m.len());

        // Supersede any in-flight decode.
        let decode_id = self.decode_job_id.get().wrapping_add(1);
        self.decode_job_id.set(decode_id);
        // Also invalidate any in-flight OCR from the previous image: its late
        // result would otherwise clear the busy spinner mid-decode (and, on a
        // failed load, repopulate the cleared canvas).
        self.ocr_job_id.set(self.ocr_job_id.get().wrapping_add(1));

        // The previous image stays visible under the busy spinner until the
        // decode finishes; the spinner then carries through into OCR.
        self.overlay.set_ocr_busy(true);

        let this = self.clone();
        let path = path.to_path_buf();
        glib::MainContext::default().spawn_local(async move {
            let result = crate::decode::decode_texture(&path).await;
            this.finish_decode(decode_id, path, name, size_bytes, result);
        });
    }

    /// Apply a finished decode, unless a newer `load_file` superseded it.
    fn finish_decode(
        &self,
        job_id: u64,
        path: PathBuf,
        name: String,
        size_bytes: Option<u64>,
        result: anyhow::Result<gtk::gdk::Texture>,
    ) {
        if job_id != self.decode_job_id.get() {
            // Late result for a file the user already navigated away from.
            return;
        }

        match result {
            Ok(texture) => {
                let info = FileInfo {
                    name,
                    width: texture.width(),
                    height: texture.height(),
                    size_bytes,
                    load_failed: false,
                };
                self.overlay.set_texture(texture);
                self.emit_file_loaded(info);
                self.start_ocr(path);
            }
            Err(err) => {
                tracing::error!("Failed to load image: {err:#}");
                // Clear the stale image (and its OCR state) so the canvas
                // matches the load_failed info shown in the headerbar.
                self.overlay.clear_texture();
                self.overlay.set_ocr_busy(false);
                self.emit_file_loaded(FileInfo {
                    name,
                    width: 0,
                    height: 0,
                    size_bytes,
                    load_failed: true,
                });
            }
        }
    }

    pub fn next_image(&self) {
        let imgs = self.dir_images.borrow();
        if imgs.is_empty() {
            return;
        }
        let mut idx = self.dir_index.get();
        idx = (idx + 1).min(imgs.len() - 1);
        self.dir_index.set(idx);
        let p = imgs[idx].clone();
        drop(imgs);
        self.load_file(&p);
    }

    pub fn prev_image(&self) {
        let imgs = self.dir_images.borrow();
        if imgs.is_empty() {
            return;
        }
        let old_idx = self.dir_index.get();
        let new_idx = old_idx.saturating_sub(1);
        if new_idx == old_idx {
            return;
        }
        self.dir_index.set(new_idx);
        let p = imgs[new_idx].clone();
        drop(imgs);
        self.load_file(&p);
    }

    pub fn copy_selection_to_clipboard(&self) {
        self.overlay.copy_selection_to_clipboard();
    }

    fn start_ocr(&self, path: PathBuf) {
        self.overlay.set_ocr_busy(true);
        self.overlay.set_ocr_result(None);

        let ocr_opts = self.ocr.borrow().clone();

        let (sender, receiver) = async_channel::bounded::<(
            u64,
            anyhow::Result<quickview_core::ocr::models::OcrResult>,
        )>(1);

        // Bump job id
        let new_id = self.ocr_job_id.get().wrapping_add(1);
        self.ocr_job_id.set(new_id);

        // All cache I/O stays on the worker thread; hits flow through the same
        // channel as fresh results, so the job-id guard applies unchanged.
        let cache_root = cache::cache_dir();
        std::thread::spawn(move || {
            let r = (|| {
                // Snapshot the cache key before OCR runs: if the file is
                // edited mid-OCR, the stale result lands under the old key,
                // which the edited file then correctly misses.
                let entry = cache_root
                    .as_deref()
                    .map(|root| cache::ocr_cache_path(root, &path, &ocr_opts));
                if let Some(cached) = entry.as_deref().and_then(cache::load_ocr) {
                    tracing::debug!("OCR cache hit for {}", path.display());
                    return Ok(cached);
                }
                let tsv_out = tesseract::run_tesseract_tsv(&path, &ocr_opts)?;
                let parsed = tsv::parse_tesseract_tsv(&tsv_out)?;
                // Empty results are cached too (text-free images shouldn't
                // re-run tesseract); failures are not, so transient errors
                // retry on the next open.
                if let Some(entry) = &entry {
                    if let Err(err) = cache::store_ocr(entry, &parsed) {
                        tracing::warn!("failed to write OCR cache: {err:#}");
                    }
                }
                Ok(parsed)
            })();

            let _ = sender.send_blocking((new_id, r));
        });

        // UI update on main thread
        let overlay = self.overlay.clone();
        let job_id_cell = self.ocr_job_id.clone();
        glib::MainContext::default().spawn_local(async move {
            if let Ok((job_id, result)) = receiver.recv().await {
                if job_id != job_id_cell.get() {
                    // Late result from previous file; ignore.
                    return;
                }

                overlay.set_ocr_busy(false);
                match result {
                    Ok(ocr) => overlay.set_ocr_result(Some(ocr)),
                    Err(err) => {
                        tracing::warn!("OCR failed: {err:#}");
                        overlay.set_ocr_result(None);
                    }
                }
            }
        });
    }
}

fn compute_dir_index(file: &Path) -> (Vec<PathBuf>, usize) {
    let dir = file.parent();
    let Some(dir) = dir else {
        return (Vec::new(), 0);
    };

    let imgs = fs::list_images_in_dir(dir).unwrap_or_default();
    let mut idx = 0;
    for (i, p) in imgs.iter().enumerate() {
        if p == file {
            idx = i;
            break;
        }
    }
    (imgs, idx)
}
