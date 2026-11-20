use image::{ImageEncoder, codecs::png, imageops::FilterType};
use std::{error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() != 2 {
        return Err(
            "usage: compress_simulated_avatars <source-directory> <output-directory>".into(),
        );
    }
    let source = PathBuf::from(&arguments[0]).canonicalize()?;
    let output = PathBuf::from(&arguments[1]);
    fs::create_dir_all(&output)?;
    let output = output.canonicalize()?;
    if output == source || output.starts_with(&source) {
        return Err("output must be independent of the source directory".into());
    }
    let mut original_bytes = 0;
    let mut compressed_bytes = 0;
    for number in 1..=30 {
        let filename = if number == 1 {
            "student-avatar-01-default.png".to_owned()
        } else {
            format!("student-avatar-{number:02}.png")
        };
        let original = fs::read(source.join(&filename))?;
        let image = image::load_from_memory(&original)?
            .resize_exact(128, 128, FilterType::Lanczos3)
            .to_rgb8();
        let mut compressed = Vec::new();
        png::PngEncoder::new_with_quality(
            &mut compressed,
            png::CompressionType::Best,
            png::FilterType::Adaptive,
        )
        .write_image(image.as_raw(), 128, 128, image::ExtendedColorType::Rgb8)?;
        original_bytes += original.len();
        compressed_bytes += compressed.len();
        println!(
            "{filename}: {} -> {} bytes",
            original.len(),
            compressed.len()
        );
        fs::write(output.join(filename), compressed)?;
    }
    println!("30 avatars: {original_bytes} -> {compressed_bytes} bytes");
    Ok(())
}
