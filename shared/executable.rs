//! Resolve an executable name or path without a shell.
use std::path::{Path, PathBuf};

pub fn resolve(
    input: &str,
    preferred_roots: impl IntoIterator<Item = PathBuf>,
) -> Result<PathBuf, String> {
    let input = input.trim().trim_matches('"');
    if input.is_empty() || input.len() > 4096 || input.contains(['\n', '\r', '\0', '"']) {
        return Err("Enter an executable name or full path, without arguments".into());
    }
    let path = Path::new(input);
    let explicit = path.is_absolute() || path.components().count() > 1;
    let search_path = std::env::var_os("PATH").unwrap_or_default();
    let roots: Vec<_> = if explicit {
        vec![PathBuf::new()]
    } else {
        preferred_roots
            .into_iter()
            .chain(std::env::split_paths(&search_path))
            .filter(|root| root.is_absolute())
            .collect()
    };
    for root in roots {
        let candidate = root.join(path);
        #[cfg(windows)]
        let candidates = if candidate.extension().is_none() {
            vec![
                candidate.with_extension("exe"),
                candidate.with_extension("com"),
            ]
        } else {
            vec![candidate]
        };
        #[cfg(not(windows))]
        let candidates = [candidate];
        for candidate in candidates {
            let Ok(metadata) = candidate.metadata() else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o111 == 0 {
                    continue;
                }
            }
            if let Ok(resolved) = candidate.canonicalize() {
                return Ok(display_path(resolved));
            }
        }
    }
    Err("Executable not found. Choose its full path or add its directory to PATH".into())
}

fn display_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    if let Some(value) = path.to_str() {
        if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{unc}"));
        }
        if let Some(disk) = value.strip_prefix(r"\\?\")
            && disk.as_bytes().get(1) == Some(&b':')
            && disk.encode_utf16().count() < 260
        {
            return PathBuf::from(disk);
        }
    }
    path
}
