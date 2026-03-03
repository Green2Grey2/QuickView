use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::Rc,
};

use gtk::prelude::*;
use gtk4 as gtk;

use quickview_core::{
    fs,
    ocr::{tesseract, tsv},
};

use crate::widgets::image_overlay::ImageOverlayWidget;

#[derive(Clone)]
pub struct ViewerController {
    overlay: ImageOverlayWidget,

    current_file: Rc<RefCell<PathBuf>>,
    dir_images: Rc<RefCell<Vec<PathBuf>>>,
    dir_index: Rc<Cell<usize>>,

    ocr_lang: Rc<RefCell<String>>,

    // Monotonic id to ignore late OCR results.
    ocr_job_id: Rc<Cell<u64>>,
}

impl ViewerController {
    pub fn new(initial_file: PathBuf, ocr_lang: String) -> Self {
        let overlay = ImageOverlayWidget::new();

        let current_file = Rc::new(RefCell::new(initial_file.clone()));
        let (dir_images, dir_index) = compute_dir_index(&initial_file);

        let this = Self {
            overlay,
            current_file,
            dir_images: Rc::new(RefCell::new(dir_images)),
            dir_index: Rc::new(Cell::new(dir_index)),
            ocr_lang: Rc::new(RefCell::new(ocr_lang)),
            ocr_job_id: Rc::new(Cell::new(0)),
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

    #[allow(dead_code)]
    pub fn current_file(&self) -> PathBuf {
        self.current_file.borrow().clone()
    }

    pub fn load_file(&self, path: &Path) {
        *self.current_file.borrow_mut() = path.to_path_buf();

        // Load image synchronously for now.
        let file = gtk::gio::File::for_path(path);
        match gtk::gdk::Texture::from_file(&file) {
            Ok(texture) => {
                self.overlay.set_texture(texture);
            }
            Err(err) => {
                tracing::error!("Failed to load image: {err}");
                // Clear OCR state.
                self.overlay.set_ocr_result(None);
                self.overlay.set_ocr_busy(false);
                return;
            }
        }

        self.start_ocr(path.to_path_buf());
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

    pub fn copy_selection_to_clipboard(&self, display: &gtk::gdk::Display) {
        let text = self.overlay.selected_text();
        if text.trim().is_empty() {
            return;
        }
        display.clipboard().set_text(&text);
    }

    fn start_ocr(&self, path: PathBuf) {
        self.overlay.set_ocr_busy(true);
        self.overlay.set_ocr_result(None);

        let lang = self.ocr_lang.borrow().clone();

        let (sender, receiver) = async_channel::bounded::<(
            u64,
            anyhow::Result<quickview_core::ocr::models::OcrResult>,
        )>(1);

        // Bump job id
        let new_id = self.ocr_job_id.get().wrapping_add(1);
        self.ocr_job_id.set(new_id);

        std::thread::spawn(move || {
            let r = (|| {
                let tsv_out = tesseract::run_tesseract_tsv(&path, &lang)?;
                let parsed = tsv::parse_tesseract_tsv(&tsv_out)?;
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
