use super::*;

#[test]
fn normalize_custom_reaction_static_resizes_png_to_square() {
    let normalized = normalize_custom_reaction_static(
        png_source_bytes(),
        &CustomReactionCropRect {
            x: 70,
            y: 0,
            size: 180,
        },
    )
    .expect("normalize png");
    let image = image::load_from_memory(normalized.bytes.as_slice()).expect("decode png");

    assert_eq!(normalized.mime, "image/png");
    assert_eq!(image.dimensions(), (128, 128));
}

#[test]
fn animated_gif_custom_reaction_preserves_gif_mime_after_normalization() {
    let normalized = normalize_custom_reaction_gif(
        animated_gif_source_bytes(),
        &CustomReactionCropRect {
            x: 1,
            y: 0,
            size: 2,
        },
    )
    .expect("normalize gif");
    let decoder =
        image::codecs::gif::GifDecoder::new(std::io::Cursor::new(normalized.bytes.clone()))
            .expect("decode normalized gif");
    let dimensions = decoder.dimensions();
    let frame_count = decoder
        .into_frames()
        .collect_frames()
        .expect("collect normalized gif frames")
        .len();

    assert_eq!(normalized.mime, "image/gif");
    assert_eq!(dimensions, (128, 128));
    assert_eq!(frame_count, 2);
}
