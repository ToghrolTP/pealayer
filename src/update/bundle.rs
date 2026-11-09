//! Explicit deployment-owned assets. No directory scanning or caller paths.
use super::*;
use serde_json::Value;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub id: String,
    path: PathBuf,
    pub manifest: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    format: String,
    assets: Vec<Asset>,
}

fn root() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Executable directory unavailable".into())
}
fn load(root: &Path) -> Result<Vec<Asset>, String> {
    let path = root.join("software-bundle.json");
    let file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.to_string()),
    };
    let mut bytes = Vec::new();
    file.take(65537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Err("Software bundle catalog exceeds 64 KiB".into());
    }
    let catalog: Catalog =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid software bundle: {e}"))?;
    if catalog.format != "digitalogic-software-bundle" || catalog.assets.len() > 16 {
        return Err("Invalid software bundle format or asset count".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for asset in &catalog.assets {
        if asset.id.is_empty()
            || asset.id.len() > 96
            || !asset
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
            || !ids.insert(&asset.id)
        {
            return Err("Software bundle IDs must be unique bounded identifiers".into());
        }
        if !matches!(
            asset.manifest["service"].as_str(),
            Some("pealayer" | "pccontroller")
        ) {
            return Err("Software bundle product is invalid".into());
        }
        normalize_sha256(
            asset.manifest["sha256"]
                .as_str()
                .ok_or("Software bundle SHA-256 required")?,
        )?;
        validate_update_size(
            asset.manifest["size"]
                .as_u64()
                .ok_or("Software bundle size required")?,
        )?;
        resolve(root, asset)?;
    }
    Ok(catalog.assets)
}
fn resolve(root: &Path, asset: &Asset) -> Result<PathBuf, String> {
    if asset.path.as_os_str().is_empty()
        || asset
            .path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err("Software bundle paths must stay inside the deployment directory".into());
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let path = root
        .join(&asset.path)
        .canonicalize()
        .map_err(|_| "Software bundle asset is unavailable")?;
    if !path.starts_with(&root) || !path.is_file() {
        return Err("Software bundle path escapes its deployment".into());
    }
    Ok(path)
}
pub fn list() -> Result<Value, String> {
    list_in(&root()?)
}
fn list_in(root: &Path) -> Result<Value, String> {
    Ok(
        serde_json::json!({"assets":load(root)?.into_iter().map(|a|serde_json::json!({"id":a.id,"manifest":a.manifest})).collect::<Vec<_>>()}),
    )
}

pub(super) fn open(id: &str) -> Result<(Value, File), String> {
    open_in(&root()?, id)
}
fn open_in(root: &Path, id: &str) -> Result<(Value, File), String> {
    let asset = load(root)?
        .into_iter()
        .find(|a| a.id == id)
        .ok_or("Software bundle asset not found")?;
    let mut file = File::open(resolve(root, &asset)?).map_err(|e| e.to_string())?;
    let expected = normalize_sha256(asset.manifest["sha256"].as_str().unwrap())?;
    let mut hash = Sha256::new();
    let mut bytes = [0; 65536];
    let mut size = 0;
    loop {
        let n = file.read(&mut bytes).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&bytes[..n]);
        size += n as u64;
        validate_update_size(size)?;
    }
    if size != asset.manifest["size"].as_u64().unwrap() || sha256_hex(hash.finalize()) != expected {
        return Err("Software bundle asset size or SHA-256 mismatch".into());
    }
    Ok((asset.manifest, file))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundle_is_explicit_confined_and_verified() {
        let root = std::env::temp_dir().join(format!("pealayer-bundle-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let bytes = b"bounded bundle fixture";
        fs::write(root.join("candidate.exe"), bytes).unwrap();
        let hash = sha256_hex(Sha256::digest(bytes));
        let catalog = |path: &str| serde_json::json!({"format":"digitalogic-software-bundle","assets":[{"id":"candidate","path":path,"manifest":{"service":"pccontroller","sha256":hash,"size":bytes.len()}}]});
        fs::write(
            root.join("software-bundle.json"),
            catalog("candidate.exe").to_string(),
        )
        .unwrap();
        assert_eq!(list_in(&root).unwrap()["assets"][0]["id"], "candidate");
        assert!(list_in(&root).unwrap()["assets"][0].get("path").is_none());
        drop(open_in(&root, "candidate").unwrap());
        fs::write(root.join("candidate.exe"), b"corrupted").unwrap();
        assert!(open_in(&root, "candidate").is_err());
        fs::write(
            root.join("software-bundle.json"),
            catalog("../candidate.exe").to_string(),
        )
        .unwrap();
        assert!(list_in(&root).is_err());
        fs::remove_file(root.join("software-bundle.json")).unwrap();
        fs::remove_file(root.join("candidate.exe")).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
