use crate::business::{Error, Result, require};
use image::{AnimationDecoder, ImageDecoder, ImageReader, Limits};
use std::io::Cursor;
pub fn validate(bytes: &[u8]) -> Result<(String, u32, u32)> {
    require(
        !bytes.is_empty() && bytes.len() <= 5 * 1024 * 1024,
        "图片不能超过 5 MiB",
    )?;
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| Error::Invalid("图片格式无效".into()))?;
    let format = reader
        .format()
        .ok_or_else(|| Error::Invalid("图片格式无效".into()))?;
    require(
        matches!(
            format,
            image::ImageFormat::Png
                | image::ImageFormat::Jpeg
                | image::ImageFormat::Gif
                | image::ImageFormat::WebP
        ),
        "仅支持 PNG/JPEG/GIF/WebP",
    )?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(100_000_000);
    reader.limits(limits.clone());
    let image = reader
        .decode()
        .map_err(|_| Error::Invalid("图片损坏或尺寸过大".into()))?;
    let (w, h) = (image.width(), image.height());
    require(
        w > 0 && h > 0 && u64::from(w) * u64::from(h) <= 24_000_000,
        "图片像素超过限制",
    )?;
    // Decode every animation frame with a bounded total pixel budget. Validating
    // only the first frame would admit broken or unbounded later frames.
    match format {
        image::ImageFormat::Gif => {
            let mut d = image::codecs::gif::GifDecoder::new(Cursor::new(bytes))
                .map_err(|_| Error::Invalid("GIF 无效".into()))?;
            d.set_limits(limits.clone())
                .map_err(|_| Error::Invalid("GIF 尺寸过大".into()))?;
            frames(d.into_frames())?;
        }
        image::ImageFormat::WebP => {
            let mut d = image::codecs::webp::WebPDecoder::new(Cursor::new(bytes))
                .map_err(|_| Error::Invalid("WebP 无效".into()))?;
            d.set_limits(limits.clone())
                .map_err(|_| Error::Invalid("WebP 尺寸过大".into()))?;
            if d.has_animation() {
                frames(d.into_frames())?;
            }
        }
        image::ImageFormat::Png => {
            let mut d = image::codecs::png::PngDecoder::new(Cursor::new(bytes))
                .map_err(|_| Error::Invalid("PNG 无效".into()))?;
            d.set_limits(limits)
                .map_err(|_| Error::Invalid("PNG 尺寸过大".into()))?;
            if d.is_apng().map_err(|_| Error::Invalid("PNG 无效".into()))? {
                frames(
                    d.apng()
                        .map_err(|_| Error::Invalid("APNG 无效".into()))?
                        .into_frames(),
                )?;
            }
        }
        _ => {}
    }
    Ok((format.to_mime_type().into(), w, h))
}
fn frames(frames: image::Frames<'_>) -> Result<()> {
    let mut pixels = 0_u64;
    for (index, frame) in frames.enumerate() {
        let frame = frame.map_err(|_| Error::Invalid("动图帧损坏".into()))?;
        pixels += u64::from(frame.buffer().width()) * u64::from(frame.buffer().height());
        require(
            index < 100 && pixels <= 100_000_000,
            "动图超过 100 帧或累计像素限制",
        )?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_headers_without_decodable_pixels() {
        assert!(validate(b"\x89PNG\r\n\x1a\n").is_err());
        let img = image::DynamicImage::new_rgb8(3, 2);
        let mut b = Cursor::new(Vec::new());
        img.write_to(&mut b, image::ImageFormat::Png).unwrap();
        assert_eq!(validate(b.get_ref()).unwrap(), ("image/png".into(), 3, 2));
    }
}
