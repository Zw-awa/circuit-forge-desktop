use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use super::types::SkinManifest;

pub fn pack_skin(manifest: &SkinManifest, asset_dir: &Path, output_path: &Path) -> Result<(), String> {
    let file = std::fs::File::create(output_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let manifest_json = serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())?;
    zip.start_file("manifest.json", options).map_err(|e| e.to_string())?;
    zip.write_all(manifest_json.as_bytes()).map_err(|e| e.to_string())?;

    for entry in std::fs::read_dir(asset_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            let name = path.file_name().unwrap().to_string_lossy();
            zip.start_file(name.as_ref(), options).map_err(|e| e.to_string())?;
            let mut buf = Vec::new();
            std::fs::File::open(&path).map_err(|e| e.to_string())?.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            zip.write_all(&buf).map_err(|e| e.to_string())?;
        }
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn unpack_skin(path: &Path) -> Result<(SkinManifest, HashMap<String, Vec<u8>>), String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut assets = HashMap::new();
    let mut manifest: Option<SkinManifest> = None;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;

        if name == "manifest.json" {
            manifest = Some(serde_json::from_slice(&buf).map_err(|e| e.to_string())?);
        } else {
            assets.insert(name, buf);
        }
    }
    let manifest = manifest.ok_or("manifest.json not found in skin pack")?;
    Ok((manifest, assets))
}
