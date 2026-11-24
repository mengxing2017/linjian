use clap::Parser;
use image::{ImageEncoder, codecs::png};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

type Result<Value> = std::result::Result<Value, Box<dyn Error>>;

#[derive(Parser)]
struct Arguments {
    #[arg(long)]
    source: PathBuf,
    #[arg(long)]
    selection: PathBuf,
    #[arg(long)]
    staging: PathBuf,
}

#[derive(Deserialize)]
struct Selection {
    files: Vec<String>,
}

#[derive(Serialize)]
struct Asset {
    file: String,
    source_sha256: String,
    sha256: String,
    original_bytes: usize,
    compressed_bytes: usize,
    width: u32,
    height: u32,
    rgba_identical: bool,
    filter: String,
}

#[derive(Serialize)]
struct Manifest {
    schema_version: u32,
    source: &'static str,
    tool: &'static str,
    parameters: &'static str,
    original_bytes: usize,
    compressed_bytes: usize,
    assets: Vec<Asset>,
}

fn filename_valid(filename: &str) -> bool {
    filename.strip_suffix(".png").is_some_and(|stem| {
        stem.split('-').all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit())
        })
    })
}

fn validate_selection(files: &[String]) -> Result<()> {
    let mut seen = HashSet::new();
    if files.is_empty() {
        return Err("selection must not be empty".into());
    }
    for filename in files {
        if !filename_valid(filename) || !seen.insert(filename) {
            return Err(format!("invalid or duplicate filename: {filename}").into());
        }
    }
    Ok(())
}

fn source_path(source: &Path, filename: &str) -> Result<PathBuf> {
    let resolved = source.join(filename).canonicalize()?;
    if !resolved.starts_with(source) || !resolved.is_file() {
        return Err(format!("source escapes directory or is not a file: {filename}").into());
    }
    Ok(resolved)
}

fn decode(bytes: &[u8]) -> Result<image::RgbaImage> {
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(512);
    limits.max_image_height = Some(512);
    limits.max_alloc = Some(8 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader.decode()?;
    if !matches!(
        decoded.color(),
        image::ColorType::L8
            | image::ColorType::La8
            | image::ColorType::Rgb8
            | image::ColorType::Rgba8
    ) {
        return Err(
            "only 8-bit PNGs are accepted; never quantize higher-precision source pixels".into(),
        );
    }
    Ok(decoded.to_rgba8())
}

fn verify(original: &[u8], compressed: &[u8]) -> Result<()> {
    if compressed.is_empty() || compressed.len() >= original.len() {
        return Err(
            "compressed icon must be strictly smaller; original fallback is forbidden".into(),
        );
    }
    if decode(original)? != decode(compressed)? {
        return Err("decoded RGBA pixels or dimensions changed".into());
    }
    Ok(())
}

fn compress(original: &[u8]) -> Result<(Vec<u8>, String)> {
    let pixels = decode(original)?;
    let mut best: Option<(Vec<u8>, String)> = None;
    for filter in [
        png::FilterType::Adaptive,
        png::FilterType::NoFilter,
        png::FilterType::Sub,
        png::FilterType::Up,
        png::FilterType::Avg,
        png::FilterType::Paeth,
    ] {
        let mut candidate = Vec::new();
        png::PngEncoder::new_with_quality(&mut candidate, png::CompressionType::Best, filter)
            .write_image(
                pixels.as_raw(),
                pixels.width(),
                pixels.height(),
                image::ExtendedColorType::Rgba8,
            )?;
        if best
            .as_ref()
            .is_none_or(|(bytes, _)| candidate.len() < bytes.len())
        {
            best = Some((candidate, format!("{filter:?}")));
        }
    }
    let (compressed, filter) = best.ok_or("no compression candidate")?;
    verify(original, &compressed)?;
    Ok((compressed, filter))
}

fn run(arguments: Arguments) -> Result<()> {
    let source = arguments.source.canonicalize()?;
    if !source.is_dir() {
        return Err("source must be a directory".into());
    }
    let selection: Selection = serde_json::from_slice(&fs::read(arguments.selection)?)?;
    validate_selection(&selection.files)?;
    let parent = arguments
        .staging
        .parent()
        .ok_or("staging needs a parent directory")?
        .canonicalize()?;
    let temporary = std::env::temp_dir().canonicalize()?;
    if !parent.starts_with(&temporary) || parent.starts_with(&source) || arguments.staging.exists()
    {
        return Err(
            "staging must be a new directory under the system temporary directory, outside source"
                .into(),
        );
    }
    let mut outputs = Vec::new();
    let mut assets = Vec::new();
    let mut preview = image::RgbaImage::new(1152, (selection.files.len().div_ceil(2) * 96) as u32);
    for filename in selection.files {
        let path = source_path(&source, &filename)?;
        let original = fs::read(&path)?;
        let (compressed, filter) =
            compress(&original).map_err(|cause| format!("{filename}: {cause}"))?;
        let pixels = decode(&compressed)?;
        let index = assets.len() as u32;
        let original_pixels = decode(&original)?;
        for (background_index, color) in
            [[255, 255, 255, 255], [31, 41, 55, 255], [91, 143, 249, 255]]
                .into_iter()
                .enumerate()
        {
            for (variant, image) in [&original_pixels, &pixels].into_iter().enumerate() {
                let horizontal = index % 2 * 576 + (background_index * 2 + variant) as u32 * 96;
                let vertical = index / 2 * 96;
                let tile = image::RgbaImage::from_pixel(96, 96, image::Rgba(color));
                image::imageops::overlay(&mut preview, &tile, horizontal.into(), vertical.into());
                let thumb = image::imageops::thumbnail(image, 80, 80);
                image::imageops::overlay(
                    &mut preview,
                    &thumb,
                    (horizontal + 8).into(),
                    (vertical + 8).into(),
                );
            }
        }
        if fs::read(&path)? != original {
            return Err(format!("source changed while compressing: {filename}").into());
        }
        assets.push(Asset {
            file: filename.clone(),
            source_sha256: format!("{:x}", Sha256::digest(&original)),
            sha256: format!("{:x}", Sha256::digest(&compressed)),
            original_bytes: original.len(),
            compressed_bytes: compressed.len(),
            width: pixels.width(),
            height: pixels.height(),
            rgba_identical: true,
            filter,
        });
        outputs.push((filename, compressed));
    }
    let manifest = Manifest {
        schema_version: 1,
        source: "wxrobot/uniapp/static/iconpark",
        tool: "compress_miniapp_icons v1 / image 0.25.10 (backend/Cargo.lock)",
        parameters: "PNG Best; smallest of Adaptive/NoFilter/Sub/Up/Avg/Paeth; RGBA8; no resize or quantization",
        original_bytes: assets.iter().map(|asset| asset.original_bytes).sum(),
        compressed_bytes: assets.iter().map(|asset| asset.compressed_bytes).sum(),
        assets,
    };
    fs::create_dir(&arguments.staging)?;
    let write = || -> Result<()> {
        for (filename, bytes) in &outputs {
            fs::write(arguments.staging.join(filename), bytes)?;
        }
        fs::write(
            arguments.staging.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        preview.save(arguments.staging.join("preview.png"))?;
        Ok(())
    };
    if let Err(cause) = write() {
        for (filename, _) in &outputs {
            let _ = fs::remove_file(arguments.staging.join(filename));
        }
        let _ = fs::remove_file(arguments.staging.join("manifest.json"));
        let _ = fs::remove_file(arguments.staging.join("preview.png"));
        let _ = fs::remove_dir(&arguments.staging);
        return Err(cause);
    }
    println!(
        "{} icons: {} -> {} bytes; RGBA identical; staging only: {}",
        manifest.assets.len(),
        manifest.original_bytes,
        manifest.compressed_bytes,
        arguments.staging.display()
    );
    Ok(())
}

fn main() -> Result<()> {
    run(Arguments::parse())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let pixels = image::RgbaImage::from_fn(96, 96, |horizontal, vertical| {
            image::Rgba([
                horizontal as u8,
                vertical as u8,
                20,
                if horizontal > 40 { 255 } else { 0 },
            ])
        });
        let mut bytes = Vec::new();
        png::PngEncoder::new_with_quality(
            &mut bytes,
            png::CompressionType::Fast,
            png::FilterType::NoFilter,
        )
        .write_image(pixels.as_raw(), 96, 96, image::ExtendedColorType::Rgba8)
        .unwrap();
        bytes
    }

    #[test]
    fn compression_preserves_every_rgba_pixel_and_reduces_size() {
        let original = fixture();
        let (compressed, _) = compress(&original).unwrap();
        verify(&original, &compressed).unwrap();
        assert!(compressed.len() < original.len());
    }

    #[test]
    fn rejects_equal_larger_empty_corrupt_and_changed_images() {
        let original = fixture();
        assert!(verify(&original, &original).is_err());
        assert!(verify(&original, &[original.as_slice(), &[0]].concat()).is_err());
        assert!(verify(&original, &[]).is_err());
        assert!(compress(b"not a png").is_err());
        assert!(verify(&original, b"not a png").is_err());
        let mut changed = decode(&original).unwrap();
        changed.put_pixel(0, 0, image::Rgba([255, 255, 255, 255]));
        let mut bytes = Vec::new();
        png::PngEncoder::new(&mut bytes)
            .write_image(changed.as_raw(), 96, 96, image::ExtendedColorType::Rgba8)
            .unwrap();
        assert!(verify(&original, &bytes).is_err());
    }

    #[test]
    fn selection_rejects_empty_duplicates_and_unsafe_names() {
        assert!(validate_selection(&[]).is_err());
        for filename in ["../home.png", "/home.png", "home.jpg", "", "home\\icon.png"] {
            assert!(validate_selection(&[filename.into()]).is_err());
        }
        assert!(validate_selection(&["home.png".into(), "home.png".into()]).is_err());
        assert!(validate_selection(&["home-neutral.png".into()]).is_ok());
    }

    #[test]
    fn higher_precision_png_is_not_silently_quantized() {
        let image = image::DynamicImage::ImageLuma16(image::ImageBuffer::from_pixel(
            2,
            2,
            image::Luma([12345_u16]),
        ));
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        assert!(compress(bytes.get_ref()).is_err());
    }

    #[test]
    fn staging_must_be_new_outside_source_and_have_an_existing_parent() {
        let root =
            std::env::temp_dir().join(format!("linjian-icons-test-{}", uuid::Uuid::new_v4()));
        let source = root.join("source");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("home.png"), fixture()).unwrap();
        let selection = root.join("selection.json");
        fs::write(&selection, r#"{"files":["home.png"]}"#).unwrap();
        for staging in [source.clone(), source.join("new"), root.join("missing/new")] {
            assert!(
                run(Arguments {
                    source: source.clone(),
                    selection: selection.clone(),
                    staging
                })
                .is_err()
            );
        }
        assert!(!source.join("new").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_batch_does_not_write_staging_or_change_source() {
        let root =
            std::env::temp_dir().join(format!("linjian-icons-test-{}", uuid::Uuid::new_v4()));
        let source = root.join("source");
        fs::create_dir_all(&source).unwrap();
        let original = fixture();
        fs::write(source.join("home.png"), &original).unwrap();
        let selection = root.join("selection.json");
        fs::write(&selection, r#"{"files":["home.png","missing.png"]}"#).unwrap();
        let staging = root.join("staging");
        assert!(
            run(Arguments {
                source: source.clone(),
                selection,
                staging: staging.clone()
            })
            .is_err()
        );
        assert!(!staging.exists());
        assert_eq!(fs::read(source.join("home.png")).unwrap(), original);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_outside_source() {
        let root =
            std::env::temp_dir().join(format!("linjian-icons-test-{}", uuid::Uuid::new_v4()));
        let source = root.join("source");
        fs::create_dir_all(&source).unwrap();
        fs::write(root.join("outside.png"), fixture()).unwrap();
        std::os::unix::fs::symlink(root.join("outside.png"), source.join("home.png")).unwrap();
        assert!(source_path(&source.canonicalize().unwrap(), "home.png").is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
