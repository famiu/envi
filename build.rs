use cfg_aliases::cfg_aliases;

fn main() {
    // The script doesn't depend on our code
    println!("cargo::rerun-if-changed=build.rs");

    // Setup cfg aliases
    cfg_aliases! {
        platform_macos: { target_os = "macos" },
        platform_windows: { target_os = "windows" },
        platform_linux: { target_os = "linux" },
    }
}
