use crate::ImageFormatExt;

use std::{collections::HashMap, path::{Path, PathBuf}};

use anyhow::{Context, Result};
use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::{DynamicImage, ImageFormat, RgbaImage};
use rayon::prelude::*;
use resvg::{tiny_skia, usvg};

pub fn image_to_other(
    images: HashMap<PathBuf, (String, bool)>,
    convert_img_format: HashMap<ImageFormatExt, bool>,
) {

}

fn ico_to_other(input_path: PathBuf, output_path: PathBuf, image_format: ImageFormatExt) -> Result<()> {

    Ok(())
}

fn svg_to_other(input_path: PathBuf, output_path: PathBuf, size: u32, image_format: ImageFormatExt) -> Result<()> {
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();

    let opt = usvg::Options {
        resources_dir: Some(input_path.clone()),
        fontdb: fontdb.into(),
        ..Default::default()
    };

    let svg_data = std::fs::read(&input_path)
        .with_context(|| format!("Failed to read file '{input_path:?}'"))?;
    let rtree = usvg::Tree::from_data(&svg_data, &opt)
        .with_context(|| "Failed to parse SVG contents")?;

    let mut pixmap = tiny_skia::Pixmap::new(size, size)
        .ok_or_else(|| anyhow::anyhow!("Failed to create SVG Pixmap!"))?;
    let pixmap_size = rtree.size();

    let transform = tiny_skia::Transform::from_scale(
        size as f32 / pixmap_size.width(),
        size as f32 / pixmap_size.height(),
    );
    resvg::render(&rtree, transform, &mut pixmap.as_mut());

    let mut image = RgbaImage::new(size, size);
    let buffer = image.as_mut();
    buffer.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
        let x = (i as u32) % size;
        let y = (i as u32) / size;

        let pixel = pixmap
            .pixel(x, y)
            .unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT);

        chunk[0] = pixel.red();
        chunk[1] = pixel.green();
        chunk[2] = pixel.blue();
        chunk[3] = pixel.alpha();
    });

    match image_format {
        ImageFormatExt::Ico => other_to_icon(image.into(), output_path, vec![16, 32, 48, 64, 128, 256])?,
        _ => image.save_with_format(output_path, image_format.get_format().unwrap_or(ImageFormat::Png))?
    }
    Ok(())
}

fn other_to_svg(input_path: &Path, output_path: &Path, config: vtracer::Config ) -> Result<()> {
    if let Err(err) = vtracer::convert_image_to_svg(input_path, output_path, config) {
        Err(anyhow::anyhow!("{err}"))
    } else {
        Ok(())
    }
}

fn other_to_icon(image: DynamicImage, output_path: PathBuf, sizes: Vec<u32>) -> Result<()> {
    let filter = image::imageops::FilterType::Lanczos3;

    let frames: Vec<IcoFrame> = sizes
        .par_iter()
        .map(|&sz| {
            let resized_image = image.resize_exact(sz, sz, filter);
            let rgba = resized_image.to_rgba8();
            IcoFrame::as_png(&rgba, sz, sz, image.color().into())
                .with_context(|| "Failed to encode frame")
        })
        .collect::<Result<Vec<IcoFrame>>>()?;

    let file = std::fs::File::create(&output_path)
        .with_context(|| format!("Failed to create file '{output_path:?}'"))?;
    
    IcoEncoder::new(file)
        .encode_images(frames.as_slice())
        .with_context(|| "Failed to encode .ico file")?;

    Ok(())
}