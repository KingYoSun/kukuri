#[path = "../../../apps/desktop/src-tauri/src/appimage_env.rs"]
mod appimage_env;

unsafe extern "C" {
    fn gio_probe() -> i32;
}

fn main() {
    // SAFETY: 単一スレッドの検証processで、GIO初期化より先に呼ぶ。
    if let Err(error) = unsafe { appimage_env::configure() } {
        eprintln!("{error}");
        std::process::exit(2);
    }
    // SAFETY: 検証専用のC関数。引数・所有権の受渡しはない。
    std::process::exit(unsafe { gio_probe() });
}
