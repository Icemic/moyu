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

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let version = env!("CARGO_PKG_VERSION")
            .split_terminator('-')
            .next()
            .unwrap();
        let version = format!("{}.{}", version, "0");
        let version_segment = version.replace(".", ",");

        embed_resource::compile(
            "static/res.rc",
            &[
                format!("VERSION=\"{version}\""),
                format!("VERSION_SEGMENT={version_segment}"),
            ],
        )
        .manifest_required()
        .unwrap();
    }
}
