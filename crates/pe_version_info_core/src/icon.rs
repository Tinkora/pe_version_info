use crate::config::{IconConfig, IconFit};
use crate::error::CoreError;
use image::imageops::{FilterType, overlay, resize};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::fs;
use std::io::{BufReader, Cursor};

pub const MAX_ICON_SOURCE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ICON_DIMENSION: u32 = 8192;
const MAX_ICON_PIXELS: u64 = 64 * 1024 * 1024;
const MAX_ICON_TARGET_SIZES: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconArtifact {
    pub ico: Vec<u8>,
    pub source_format: String,
    pub renderer: String,
    pub target_sizes: Vec<u16>,
    pub cropped: bool,
}

pub fn convert_icon(config: &IconConfig) -> Result<IconArtifact, CoreError> {
    if config.fit == IconFit::Cover && !config.allow_crop {
        return Err(CoreError::IconCropNotAllowed);
    }
    validate_target_sizes(&config.target_sizes)?;
    let metadata = fs::metadata(&config.source).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => CoreError::PathNotFound(config.source.clone()),
        _ => CoreError::PathNotRegularFile(config.source.clone()),
    })?;
    if !metadata.is_file() {
        return Err(CoreError::PathNotRegularFile(config.source.clone()));
    }
    if metadata.len() > MAX_ICON_SOURCE_BYTES {
        return Err(CoreError::IconSourceTooLarge);
    }
    let bytes = fs::read(&config.source).map_err(|_| CoreError::IconInvalid)?;
    let format =
        ImageFormat::from_path(&config.source).map_err(|_| CoreError::UnsupportedInputExtension)?;
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Ico
    ) {
        return Err(CoreError::UnsupportedInputExtension);
    }
    let mut reader = image::ImageReader::with_format(BufReader::new(Cursor::new(&bytes)), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_ICON_DIMENSION);
    limits.max_image_height = Some(MAX_ICON_DIMENSION);
    limits.max_alloc = Some(MAX_ICON_PIXELS.saturating_mul(4));
    reader.limits(limits);
    let source = reader.decode().map_err(|_| CoreError::IconInvalid)?;
    validate_dimensions(&source)?;
    let source = source.to_rgba8();
    let mut frames = Vec::with_capacity(config.target_sizes.len());
    for &size in &config.target_sizes {
        let frame = compose_frame(
            &source,
            u32::from(size),
            config.fit == IconFit::Cover,
            parse_background(&config.background)?,
        )?;
        let encoded = encode_windows_dib(&frame)?;
        frames.push((encoded, size));
    }
    let ico = encode_ico(&frames)?;
    image::load_from_memory_with_format(&ico, ImageFormat::Ico)
        .map_err(|_| CoreError::IconInvalid)?;

    Ok(IconArtifact {
        ico,
        source_format: format_name(format).to_owned(),
        renderer: "image".to_owned(),
        target_sizes: config.target_sizes.clone(),
        cropped: config.fit == IconFit::Cover,
    })
}

fn validate_target_sizes(sizes: &[u16]) -> Result<(), CoreError> {
    if sizes.is_empty()
        || sizes.len() > MAX_ICON_TARGET_SIZES
        || sizes.windows(2).any(|pair| pair[0] >= pair[1])
        || sizes.iter().any(|size| !(16..=256).contains(size))
    {
        return Err(CoreError::ConfigInvalid);
    }
    Ok(())
}

fn validate_dimensions(image: &DynamicImage) -> Result<(), CoreError> {
    let (width, height) = (image.width(), image.height());
    if width == 0
        || height == 0
        || width > MAX_ICON_DIMENSION
        || height > MAX_ICON_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_ICON_PIXELS
    {
        return Err(CoreError::IconInvalid);
    }
    Ok(())
}

fn compose_frame(
    source: &RgbaImage,
    size: u32,
    cover: bool,
    background: Rgba<u8>,
) -> Result<RgbaImage, CoreError> {
    let (width, height) = source.dimensions();
    let scale = if cover {
        (size as f64 / width as f64).max(size as f64 / height as f64)
    } else {
        (size as f64 / width as f64).min(size as f64 / height as f64)
    };
    let resized_width = (width as f64 * scale).round().max(1.0) as u32;
    let resized_height = (height as f64 * scale).round().max(1.0) as u32;
    let resized = resize(source, resized_width, resized_height, FilterType::Lanczos3);
    let mut canvas = RgbaImage::from_pixel(size, size, background);
    let x = (size as i64 - i64::from(resized_width)) / 2;
    let y = (size as i64 - i64::from(resized_height)) / 2;
    overlay(&mut canvas, &resized, x, y);
    Ok(canvas)
}

fn parse_background(value: &str) -> Result<Rgba<u8>, CoreError> {
    if value == "transparent" {
        return Ok(Rgba([0, 0, 0, 0]));
    }
    if value.len() != 9 || !value.starts_with('#') {
        return Err(CoreError::ConfigInvalid);
    }
    let bytes = (0..4)
        .map(|index| u8::from_str_radix(&value[1 + index * 2..3 + index * 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| CoreError::ConfigInvalid)?;
    Ok(Rgba([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn encode_windows_dib(frame: &RgbaImage) -> Result<Vec<u8>, CoreError> {
    const HEADER_SIZE: u32 = 40;
    const BYTES_PER_PIXEL: usize = 4;

    let width = frame.width();
    let height = frame.height();
    let pixel_bytes = usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(usize::try_from(height).ok()?))
        .and_then(|pixels| pixels.checked_mul(BYTES_PER_PIXEL))
        .ok_or(CoreError::IconInvalid)?;
    let mask_stride = usize::try_from(width.div_ceil(32))
        .ok()
        .and_then(|dwords| dwords.checked_mul(4))
        .ok_or(CoreError::IconInvalid)?;
    let mask_bytes = mask_stride
        .checked_mul(usize::try_from(height).map_err(|_| CoreError::IconInvalid)?)
        .ok_or(CoreError::IconInvalid)?;
    let image_bytes = pixel_bytes
        .checked_add(mask_bytes)
        .ok_or(CoreError::IconInvalid)?;
    let capacity = usize::try_from(HEADER_SIZE)
        .ok()
        .and_then(|header| header.checked_add(image_bytes))
        .ok_or(CoreError::IconInvalid)?;
    let doubled_height = height.checked_mul(2).ok_or(CoreError::IconInvalid)?;
    let mut output = Vec::with_capacity(capacity);

    output.extend_from_slice(&HEADER_SIZE.to_le_bytes());
    output.extend_from_slice(
        &i32::try_from(width)
            .map_err(|_| CoreError::IconInvalid)?
            .to_le_bytes(),
    );
    output.extend_from_slice(
        &i32::try_from(doubled_height)
            .map_err(|_| CoreError::IconInvalid)?
            .to_le_bytes(),
    );
    output.extend_from_slice(&1u16.to_le_bytes());
    output.extend_from_slice(&32u16.to_le_bytes());
    output.extend_from_slice(&0u32.to_le_bytes());
    output.extend_from_slice(
        &u32::try_from(image_bytes)
            .map_err(|_| CoreError::IconInvalid)?
            .to_le_bytes(),
    );
    output.extend_from_slice(&0i32.to_le_bytes());
    output.extend_from_slice(&0i32.to_le_bytes());
    output.extend_from_slice(&0u32.to_le_bytes());
    output.extend_from_slice(&0u32.to_le_bytes());

    for y in (0..height).rev() {
        for x in 0..width {
            let [red, green, blue, alpha] = frame.get_pixel(x, y).0;
            output.extend_from_slice(&[blue, green, red, alpha]);
        }
    }

    let mut mask_row = vec![0u8; mask_stride];
    for y in (0..height).rev() {
        mask_row.fill(0);
        for x in 0..width {
            if frame.get_pixel(x, y).0[3] == 0 {
                let byte = usize::try_from(x / 8).map_err(|_| CoreError::IconInvalid)?;
                mask_row[byte] |= 1 << (7 - (x % 8));
            }
        }
        output.extend_from_slice(&mask_row);
    }

    Ok(output)
}

fn encode_ico(frames: &[(Vec<u8>, u16)]) -> Result<Vec<u8>, CoreError> {
    let count = u16::try_from(frames.len()).map_err(|_| CoreError::IconInvalid)?;
    let directory_size = 6usize
        .checked_add(frames.len().checked_mul(16).ok_or(CoreError::IconInvalid)?)
        .ok_or(CoreError::IconInvalid)?;
    let mut output = Vec::with_capacity(
        directory_size + frames.iter().map(|(data, _)| data.len()).sum::<usize>(),
    );
    output.extend_from_slice(&[0, 0, 1, 0]);
    output.extend_from_slice(&count.to_le_bytes());
    let mut offset = u32::try_from(directory_size).map_err(|_| CoreError::IconInvalid)?;
    for (data, size) in frames {
        let dimension = if *size == 256 {
            0
        } else {
            u8::try_from(*size).map_err(|_| CoreError::IconInvalid)?
        };
        output.extend_from_slice(&[dimension, dimension, 0, 0]);
        output.extend_from_slice(&1u16.to_le_bytes());
        output.extend_from_slice(&32u16.to_le_bytes());
        output.extend_from_slice(
            &u32::try_from(data.len())
                .map_err(|_| CoreError::IconInvalid)?
                .to_le_bytes(),
        );
        output.extend_from_slice(&offset.to_le_bytes());
        offset = offset
            .checked_add(u32::try_from(data.len()).map_err(|_| CoreError::IconInvalid)?)
            .ok_or(CoreError::IconInvalid)?;
    }
    for (data, _) in frames {
        output.extend_from_slice(data);
    }
    Ok(output)
}

fn format_name(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "png",
        ImageFormat::Jpeg => "jpeg",
        ImageFormat::Ico => "ico",
        _ => "unknown",
    }
}
