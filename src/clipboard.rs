#[cfg(not(target_arch = "wasm32"))]
pub struct ImageData {
    pub bytes: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

#[cfg(not(target_arch = "wasm32"))]
pub struct Clipboard(arboard::Clipboard);

#[cfg(not(target_arch = "wasm32"))]
impl Clipboard {
    pub fn new() -> Option<Self> {
        let clipboard = match arboard::Clipboard::new() {
            Ok(c) => Some(c),
            Err(_) => {
                eprintln!("[ERROR] Couldn't initialize clipboard");
                None
            }
        }?;
        Some(Self(clipboard))
    }

    pub fn get_image(&mut self) -> Result<ImageData, arboard::Error> {
        let img = self.0.get_image()?;
        let bytes = img.bytes.into_owned();
        Ok(ImageData {
            bytes,
            width: img.width,
            height: img.height,
        })
    }
}

#[cfg(target_arch = "wasm32")]
pub struct Clipboard;

#[cfg(target_arch = "wasm32")]
pub struct ImageData;

#[cfg(target_arch = "wasm32")]
impl Clipboard {
    pub fn new() -> Option<Self> {
        None
    }

    pub fn get_image(&mut self) -> Option<Self> {
        None
    }
}
