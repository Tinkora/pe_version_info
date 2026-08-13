use editpe::ToIcon;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use pe_version_info_core::config::{IconConfig, IconFit};
use pe_version_info_core::error::CoreError;
use pe_version_info_core::icon::convert_icon;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn write_image(path: &Path, image: DynamicImage, format: ImageFormat) {
    let mut bytes = Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, format)
        .expect("test image should be encoded");
    fs::write(path, bytes.into_inner()).expect("encoded test image should be written");
}

fn config(source: PathBuf) -> IconConfig {
    IconConfig {
        source,
        fit: IconFit::Contain,
        allow_crop: false,
        background: "transparent".to_owned(),
        target_sizes: vec![16, 32, 64],
    }
}

#[test]
fn converts_png_with_transparent_contain_fit_and_expected_sizes() {
    let directory = tempdir().expect("temporary directory should be created");
    let source = directory.path().join("source.png");
    let source_image = DynamicImage::ImageRgba8(ImageBuffer::from_fn(80, 40, |x, y| {
        if x < 4 || y < 4 {
            Rgba([0, 0, 0, 0])
        } else {
            Rgba([255, 0, 0, 255])
        }
    }));
    write_image(&source, source_image, ImageFormat::Png);

    let artifact = convert_icon(&config(source)).expect("PNG should convert");

    assert_eq!(artifact.source_format.as_str(), "png");
    assert_eq!(artifact.renderer.as_str(), "image");
    assert_eq!(artifact.target_sizes, vec![16, 32, 64]);
    assert!(!artifact.cropped);
    let entries = artifact.ico.as_slice().icons().expect("ICO should parse");
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().any(|entry| entry[0] == 16));
    assert!(entries.iter().any(|entry| entry[0] == 32));
    assert!(entries.iter().any(|entry| entry[0] == 64));
}

#[test]
fn encodes_windows_compatible_dib_frames_with_and_masks() {
    let directory = tempdir().expect("temporary directory should be created");
    let source = directory.path().join("source.png");
    let source_image = DynamicImage::ImageRgba8(ImageBuffer::from_fn(80, 40, |x, _| {
        if x < 8 {
            Rgba([0, 0, 0, 0])
        } else {
            Rgba([10, 20, 30, 255])
        }
    }));
    write_image(&source, source_image, ImageFormat::Png);

    let artifact = convert_icon(&config(source)).expect("PNG should convert");
    let entries = artifact.ico.as_slice().icons().expect("ICO should parse");

    for entry in entries {
        let size = if entry[0] == 0 {
            256usize
        } else {
            usize::from(entry[0])
        };
        let payload = &entry[14..];
        let header_size = u32::from_le_bytes(payload[0..4].try_into().unwrap());
        let width = i32::from_le_bytes(payload[4..8].try_into().unwrap());
        let doubled_height = i32::from_le_bytes(payload[8..12].try_into().unwrap());
        let planes = u16::from_le_bytes(payload[12..14].try_into().unwrap());
        let bit_count = u16::from_le_bytes(payload[14..16].try_into().unwrap());
        let xor_bytes = size * size * 4;
        let and_stride = size.div_ceil(32) * 4;

        assert_eq!(header_size, 40);
        assert_eq!(width, size as i32);
        assert_eq!(doubled_height, (size * 2) as i32);
        assert_eq!(planes, 1);
        assert_eq!(bit_count, 32);
        assert_eq!(payload.len(), 40 + xor_bytes + and_stride * size);
        assert!(payload[40 + xor_bytes..].iter().any(|byte| *byte != 0));
    }
}

#[test]
fn preserves_transparent_letterbox_without_cover_crop() {
    let directory = tempdir().expect("temporary directory should be created");
    let source = directory.path().join("wide.png");
    let source_image =
        DynamicImage::ImageRgba8(ImageBuffer::from_pixel(80, 40, Rgba([0, 255, 0, 255])));
    write_image(&source, source_image, ImageFormat::Png);

    let artifact = convert_icon(&config(source)).expect("wide PNG should convert");
    let decoded = image::load_from_memory_with_format(&artifact.ico, ImageFormat::Ico)
        .expect("generated ICO should decode")
        .to_rgba8();

    assert!(!artifact.cropped);
    assert_eq!(decoded.width(), 64);
    assert_eq!(decoded.get_pixel(0, 0).0[3], 0);
    assert_eq!(decoded.get_pixel(32, 32).0[1], 255);
}

#[test]
fn uses_explicit_background_for_contain_fit() {
    let directory = tempdir().expect("temporary directory should be created");
    let source = directory.path().join("tall.png");
    let source_image =
        DynamicImage::ImageRgba8(ImageBuffer::from_pixel(40, 80, Rgba([0, 0, 255, 255])));
    write_image(&source, source_image, ImageFormat::Png);
    let mut icon = config(source);
    icon.target_sizes = vec![32];
    icon.background = "#ff00ffff".to_owned();

    let artifact = convert_icon(&icon).expect("background should be accepted");
    let decoded = image::load_from_memory_with_format(&artifact.ico, ImageFormat::Ico)
        .expect("generated ICO should decode")
        .to_rgba8();

    assert_eq!(decoded.get_pixel(0, 0).0, [255, 0, 255, 255]);
}

#[test]
fn refuses_cover_without_allow_crop() {
    let directory = tempdir().expect("temporary directory should be created");
    let source = directory.path().join("source.png");
    write_image(
        &source,
        DynamicImage::ImageRgba8(ImageBuffer::from_pixel(40, 80, Rgba([0, 0, 255, 255]))),
        ImageFormat::Png,
    );
    let mut icon = config(source);
    icon.fit = IconFit::Cover;

    assert!(matches!(
        convert_icon(&icon),
        Err(CoreError::IconCropNotAllowed)
    ));
}

#[test]
fn converts_jpeg_and_ico_sources() {
    let directory = tempdir().expect("temporary directory should be created");
    let jpeg = directory.path().join("source.jpg");
    let ico = directory.path().join("source.ico");
    let source_image =
        DynamicImage::ImageRgba8(ImageBuffer::from_pixel(32, 32, Rgba([10, 20, 30, 255])));
    write_image(&jpeg, source_image.clone(), ImageFormat::Jpeg);
    write_image(&ico, source_image, ImageFormat::Ico);

    for source in [jpeg, ico] {
        let artifact = convert_icon(&config(source)).expect("supported source should convert");
        assert_eq!(artifact.target_sizes, vec![16, 32, 64]);
        artifact
            .ico
            .as_slice()
            .icons()
            .expect("ICO should be valid");
    }
}

#[test]
fn rejects_malformed_and_oversized_sources_with_stable_errors() {
    let directory = tempdir().expect("temporary directory should be created");
    let malformed = directory.path().join("bad.png");
    fs::write(&malformed, b"not an image").expect("malformed source should be written");
    assert!(matches!(
        convert_icon(&config(malformed)),
        Err(CoreError::IconInvalid)
    ));

    let oversized = directory.path().join("large.png");
    let file = fs::File::create(&oversized).expect("oversized source should be created");
    file.set_len(pe_version_info_core::icon::MAX_ICON_SOURCE_BYTES + 1)
        .expect("oversized source should be sparse resized");
    assert!(matches!(
        convert_icon(&config(oversized)),
        Err(CoreError::IconSourceTooLarge)
    ));
}

#[test]
fn rejects_dimensions_before_large_pixel_allocation() {
    let directory = tempdir().expect("temporary directory should be created");
    let source = directory.path().join("huge-dimensions.png");
    let image = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(8193, 1, Rgba([1, 2, 3, 255])));
    write_image(&source, image, ImageFormat::Png);

    assert!(matches!(
        convert_icon(&config(source)),
        Err(CoreError::IconInvalid)
    ));
}
