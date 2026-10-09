//! One source-identity emitter for the player and standalone utility.
pub fn emit_build_metadata() {
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
    // Linked worktrees own HEAD/index but keep branch refs in the common Git directory.
    let mut git_paths = vec![
        "HEAD".to_string(),
        "index".to_string(),
        "packed-refs".to_string(),
    ];
    if let Some(reference) = git_output(&["symbolic-ref", "-q", "HEAD"]) {
        git_paths.push(reference);
    }
    for path in git_paths {
        if let Some(resolved) = git_output(&["rev-parse", "--git-path", &path]) {
            println!("cargo:rerun-if-changed={resolved}");
        }
    }
}
