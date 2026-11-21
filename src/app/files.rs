use super::*;

pub(crate) fn copy_and_save(
    img: &image::RgbaImage,
    dir: &std::path::Path,
    auto_save: bool,
) -> String {
    let mut msg = match copy_to_clipboard(img) {
        Ok(()) => tr(strings::COPIED_TO_CLIPBOARD).to_owned(),
        Err(e) => tr1(strings::COPY_FAILED, e),
    };
    if auto_save {
        match save_auto_to(dir, img) {
            Ok(p) => msg.push_str(&tr1(strings::SAVED, p.display())),
            Err(e) => msg.push_str(&tr1(strings::AUTO_SAVE_FAILED, e)),
        }
    }
    msg
}

pub(crate) fn save_auto_to(
    dir: &std::path::Path,
    img: &image::RgbaImage,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let stem = format!("RustShot_{}", crate::platform::util::timestamp());
    let path = crate::config::unique_png(dir, &stem);
    save_png(img, &path)?;
    Ok(path)
}

pub(crate) fn copy_to_clipboard(img: &image::RgbaImage) -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_image(arboard::ImageData {
        width: img.width() as usize,
        height: img.height() as usize,
        bytes: std::borrow::Cow::Borrowed(img.as_raw()),
    })
    .map_err(|e| e.to_string())
}

pub(crate) fn save_png(img: &image::RgbaImage, path: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    image::save_buffer(
        path,
        img.as_raw(),
        img.width(),
        img.height(),
        image::ColorType::Rgba8,
    )
    .map_err(|e| e.to_string())
}
