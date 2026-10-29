//! UI sounds. Kenney Interface Sounds, CC0.
//! https://kenney.nl/assets/interface-sounds

pub fn click() {
    play(include_bytes!("../assets/sfx/click.wav"));
}
pub fn select() {
    play(include_bytes!("../assets/sfx/select.wav"));
}
pub fn toggle() {
    play(include_bytes!("../assets/sfx/toggle.wav"));
}
pub fn ok() {
    play(include_bytes!("../assets/sfx/ok.wav"));
}
pub fn error() {
    play(include_bytes!("../assets/sfx/error.wav"));
}
pub fn open() {
    play(include_bytes!("../assets/sfx/open.wav"));
}
pub fn back() {
    play(include_bytes!("../assets/sfx/back.wav"));
}

fn play(bytes: &'static [u8]) {
    #[cfg(target_os = "windows")]
    {
        const SND_ASYNC: u32 = 0x0001;
        const SND_NODEFAULT: u32 = 0x0002;
        const SND_MEMORY: u32 = 0x0004;
        #[link(name = "winmm")]
        extern "system" {
            fn PlaySoundA(name: *const u8, module: *mut core::ffi::c_void, flags: u32) -> i32;
        }
        unsafe {
            PlaySoundA(bytes.as_ptr(), core::ptr::null_mut(), SND_ASYNC | SND_NODEFAULT | SND_MEMORY);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = bytes;
    }
}
