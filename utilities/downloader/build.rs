#[path = "../../scripts/build-metadata.rs"]
mod build_metadata;
#[path = "src/icon.rs"]
mod icon;
fn main() {
    build_metadata::emit_build_metadata();
    println!("cargo:rerun-if-changed=src/icon.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let output =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("downloader.ico");
    let master = image::RgbaImage::from_raw(256, 256, icon::rgba(256)).unwrap();
    let frames = [16, 20, 24, 32, 40, 48, 64, 128, 256]
        .into_iter()
        .map(|size| {
            let resized =
                image::imageops::resize(&master, size, size, image::imageops::FilterType::Lanczos3);
            image::codecs::ico::IcoFrame::as_png(
                resized.as_raw(),
                size,
                size,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    image::codecs::ico::IcoEncoder::new(std::fs::File::create(&output).unwrap())
        .encode_images(&frames)
        .unwrap();
    let mut resources = winres::WindowsResource::new();
    resources
        .set_icon(output.to_str().unwrap())
        .set("ProductName", "Pealayer Downloader")
        .set("FileDescription", "Pealayer standalone download utility")
        .set("OriginalFilename", "pealayer-downloader.exe")
        .set("ProductVersion", env!("CARGO_PKG_VERSION"))
        .set("FileVersion", env!("CARGO_PKG_VERSION"));
    resources
        .compile()
        .expect("Compile downloader executable metadata");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        let resource = output.parent().unwrap().join("resource.o");
        println!(
            "cargo:rustc-link-arg-bin=pealayer-downloader={}",
            resource.display()
        );
    }
}
