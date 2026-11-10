#[cfg(target_os = "windows")]
fn main() {
    emit_build_metadata();
    for name in [
        "APP_NAME",
        "APP_PUBLISHER",
        "APP_COPYRIGHT",
        "APP_DESCRIPTION",
        "APP_ICON_ICO",
        "APP_EXECUTABLE_NAME",
        "APPLICATION_BRAND",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    let package_version = env!("CARGO_PKG_VERSION");
    let mut version_parts = package_version
        .split_once('-')
        .map_or(package_version, |(core, _)| core)
        .split('.')
        .map(|part| part.parse::<u16>().unwrap_or(0))
        .collect::<Vec<_>>();
    version_parts.resize(4, 0);
    let windows_version = version_parts
        .iter()
        .take(4)
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(".");

    let brand_path = std::env::var_os("APPLICATION_BRAND").map(std::path::PathBuf::from);
    let brand = brand_path.as_ref().map(|path| {
        println!("cargo:rerun-if-changed={}", path.display());
        let source = std::fs::read_to_string(path).unwrap_or_else(|error| {
            panic!(
                "could not read application branding file {}: {error}",
                path.display()
            )
        });
        let value: serde_json::Value = serde_json::from_str(&source)
            .unwrap_or_else(|error| panic!("invalid branding JSON {}: {error}", path.display()));
        if let Some(format) = value.get("format").and_then(serde_json::Value::as_str) {
            assert_eq!(
                format, "application-brand",
                "unsupported application branding format"
            );
        }
        value
    });
    let brand_string = |name: &str| {
        brand
            .as_ref()
            .and_then(|value| value.get(name))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    };
    let product_name = std::env::var("APP_NAME")
        .ok()
        .or_else(|| brand_string("applicationName"))
        .unwrap_or_else(|| "Pealayer".to_string());
    let publisher = std::env::var("APP_PUBLISHER")
        .ok()
        .or_else(|| brand_string("companyName"))
        .unwrap_or_default();
    let copyright = std::env::var("APP_COPYRIGHT")
        .ok()
        .or_else(|| brand_string("legalCopyright"))
        .unwrap_or_default();
    let description = std::env::var("APP_DESCRIPTION")
        .ok()
        .or_else(|| brand_string("fileDescription"))
        .unwrap_or_else(|| product_name.clone());
    let executable_name = std::env::var("APP_EXECUTABLE_NAME")
        .ok()
        .or_else(|| brand_string("executableName"))
        .unwrap_or_else(|| "pealayer".to_string());
    assert!(
        !executable_name.is_empty()
            && executable_name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
            && !executable_name.contains(".."),
        "branding executableName must be a safe extension-free file name"
    );
    let document_icon = brand.as_ref().and_then(|value| {
        let relative = value.get("windowsIcons")?.get("APP")?.as_str()?;
        let base = brand_path
            .as_ref()?
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        Some(base.join(relative).to_string_lossy().into_owned())
    });
    let icon = std::env::var("APP_ICON_ICO")
        .ok()
        .or(document_icon)
        .unwrap_or_else(|| "assets/icon.ico".to_string());

    let mut res = winres::WindowsResource::new();
    res.set("FileVersion", &windows_version);
    res.set("ProductVersion", &windows_version);
    res.set("FileDescription", &description);
    res.set("ProductName", &product_name);
    res.set("InternalName", &executable_name);
    res.set("OriginalFilename", &format!("{executable_name}.exe"));
    if !publisher.trim().is_empty() {
        res.set("CompanyName", &publisher);
    }
    if !copyright.trim().is_empty() {
        res.set("LegalCopyright", &copyright);
    }
    if std::path::Path::new(&icon).exists() {
        res.set_icon(&icon);
    }
    res.compile().expect("failed to compile Windows resources");

    // This package exposes both a library and a binary. Resource-only archives
    // have no symbols for the executable to reference, so both GNU ld and
    // MSVC's linker may discard them. Attach the generated resource object on
    // GNU and retain the entire resource library on MSVC explicitly.
    let out_dir = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").expect("Cargo did not provide OUT_DIR"),
    );
    match std::env::var("CARGO_CFG_TARGET_ENV").as_deref() {
        Ok("gnu") => {
            let resource = out_dir.join("resource.o");
            println!("cargo:rustc-link-arg-bin=pealayer={}", resource.display());
        }
        Ok("msvc") => {
            // winres names the rc.exe output resource.lib, but it is a COFF
            // resource object rather than an archive. Pass it directly to
            // link.exe; treating it as an archive lets /OPT:REF discard it.
            let resource = out_dir.join("resource.lib");
            println!("cargo:rustc-link-arg-bin=pealayer={}", resource.display());
        }
        _ => {}
    }
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if icon != "assets/icon.ico" {
        println!("cargo:rerun-if-changed={icon}");
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    emit_build_metadata();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        if let Ok(output) = std::process::Command::new("brew")
            .args(["--prefix", "mpv"])
            .output()
        {
            if output.status.success() {
                let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let lib_dir = std::path::PathBuf::from(&prefix).join("lib");
                if lib_dir.exists() {
                    println!("cargo:rustc-link-search=native={}", lib_dir.display());
                    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
                }
            }
        }
        for path in [
            "/opt/homebrew/lib",
            "/opt/homebrew/opt/mpv/lib",
            "/usr/local/lib",
            "/usr/local/opt/mpv/lib",
        ] {
            if std::path::Path::new(path).exists() {
                println!("cargo:rustc-link-search=native={path}");
                println!("cargo:rustc-link-arg=-Wl,-rpath,{path}");
            }
        }
    }
}

fn emit_build_metadata() {
    for name in [
        "PROFILE",
        "TARGET",
        "SOURCE_DATE_EPOCH",
        "GITHUB_SHA",
        "GITHUB_REF_NAME",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }

    let git_output = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .filter(|value| !value.is_empty())
    };
    let commit = std::env::var("GITHUB_SHA")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| git_output(&["rev-parse", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_string());
    let branch = std::env::var("GITHUB_REF_NAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| git_output(&["rev-parse", "--abbrev-ref", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_string());
    let dirty = std::process::Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| !output.stdout.is_empty());
    let rustc =
        std::process::Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=PEALAYER_GIT_COMMIT={commit}");
    println!("cargo:rustc-env=PEALAYER_GIT_BRANCH={branch}");
    println!("cargo:rustc-env=PEALAYER_GIT_DIRTY={dirty}");
    println!(
        "cargo:rustc-env=PEALAYER_BUILD_PROFILE={}",
        std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string())
    );
    println!(
        "cargo:rustc-env=PEALAYER_BUILD_TARGET={}",
        std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string())
    );
    println!("cargo:rustc-env=PEALAYER_RUSTC_VERSION={rustc}");
    println!(
        "cargo:rustc-env=PEALAYER_SOURCE_DATE_EPOCH={}",
        std::env::var("SOURCE_DATE_EPOCH").unwrap_or_else(|_| "not supplied".to_string())
    );
    println!("cargo:rerun-if-changed=.git/HEAD");
    if let Some(git_dir) = git_output(&["rev-parse", "--git-dir"]) {
        let head = std::fs::read_to_string(std::path::Path::new(&git_dir).join("HEAD")).ok();
        if let Some(reference) = head.and_then(|value| {
            value
                .strip_prefix("ref: ")
                .map(str::trim)
                .map(str::to_string)
        }) {
            println!(
                "cargo:rerun-if-changed={}",
                std::path::Path::new(&git_dir).join(reference).display()
            );
        }
    }
}
