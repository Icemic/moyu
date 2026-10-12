fn main() {
    cfg_aliases::cfg_aliases! {
        linux: { target_os = "linux" },
        macos: { target_os = "macos" },
        android: { target_os = "android" },
        ios: { target_os = "ios" },
        wasm: { target_arch = "wasm32" },

        native: { any(windows, linux, macos, android, ios) },
        desktop: { any(windows, linux, macos) },
        mobile: { any(android, ios) },
        web: { any(wasm) },
    }
}
