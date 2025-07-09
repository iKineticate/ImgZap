use crate::ImageFormatExt;

use anyhow::{Context, Result};
use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::{DynamicImage, ImageFormat, RgbaImage};
use rayon::prelude::*;
use resvg::{tiny_skia, usvg};
use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
};
use vtracer::ColorImage;

pub fn image_to_other(
    images: &HashMap<PathBuf, (ImageFormatExt, bool)>,
    convert_img_format: &HashMap<ImageFormatExt, bool>,
) {
    images
        .into_iter()
        .filter_map(|(p, (f, is_check))| is_check.then_some((p, f)))
        .for_each(|(input_path, iamge_format)| {
            convert_img_format
                .into_iter()
                .filter_map(|(convert_format, is_convert)| {
                    (*is_convert && iamge_format.ne(convert_format)).then_some(convert_format)
                })
                .for_each(|convert_format| match iamge_format {
                    ImageFormatExt::Svg => {
                        let output_path = input_path.with_extension(convert_format.get_ext());
                        if svg_to_other(input_path, &output_path, 256, convert_format)
                            .inspect_err(|e| println!("Failed to svg convert to {convert_format:?}\n{input_path:?}\n{e:?}"))
                            .is_ok()
                        {};
                    }
                    ImageFormatExt::Ico => {
                        let output_path = input_path.with_extension(convert_format.get_ext());
                        if ico_to_other(input_path, &output_path, convert_format)
                            .inspect_err(|e| println!("Failed to icon convert to {convert_format:?}\n{input_path:?}\n{e:?}"))
                            .is_ok()
                        {};
                    }
                    _ => {
                        let output_path = input_path.with_extension(convert_format.get_ext());
                        if other_to_other(input_path, &output_path, convert_format)
                            .inspect_err(|e| println!("Failed to convert to {convert_format:?}\n{input_path:?}\n{e:?}"))
                            .is_ok()
                        {};
                    }
                });
        });
}

fn ico_to_other(
    input_path: &Path,
    output_path: &Path,
    convert_format: &ImageFormatExt,
) -> Result<()> {
    let file = std::fs::File::open(input_path)?;
    let icon_dir = ico::IconDir::read(file)?;
    let largest_entry = icon_dir
        .entries()
        .into_iter()
        .max_by_key(|entry| entry.width() * entry.height())
        .ok_or(anyhow::anyhow!(
            "No images found in ICO file: {input_path:?}"
        ))?;

    let ico_image = largest_entry.decode()?;

    match convert_format.get_format() {
        Some(f) => {
            let output_file = std::fs::File::create(output_path)?;
            let mut writer = std::io::BufWriter::new(output_file);

            if f == ImageFormat::Jpeg {
                let rgba_image = RgbaImage::from_raw(
                    ico_image.width() as u32,
                    ico_image.height() as u32,
                    ico_image.rgba_data().to_vec(),
                )
                .ok_or(anyhow::anyhow!(
                    "Failed to create RGBA image: {input_path:?}"
                ))?;

                let rgb_image = DynamicImage::ImageRgba8(rgba_image).to_rgb8();
                rgb_image.save_with_format(
                    output_path,
                    convert_format
                        .get_format()
                        .expect("No supported image formats"),
                )?;
            } else {
                let rgba_image = RgbaImage::from_raw(
                    ico_image.width() as u32,
                    ico_image.height() as u32,
                    ico_image.rgba_data().to_vec(),
                )
                .ok_or(anyhow::anyhow!(
                    "Failed to create RGBA image: {input_path:?}"
                ))?;

                rgba_image.write_to(&mut writer, f)?
            }
        }
        None => {
            let svg_file = vtracer::convert(
                ColorImage {
                    pixels: ico_image.rgba_data().to_vec(),
                    width: ico_image.width() as usize,
                    height: ico_image.height() as usize,
                },
                vtracer::Config::default(),
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;

            let mut output_file = std::fs::File::create(output_path)?;
            write!(&mut output_file, "{}", svg_file).with_context(|| "Failed to write file.")?;
        }
    }

    Ok(())
}

fn svg_to_other(
    input_path: &Path,
    output_path: &Path,
    size: u32,
    convert_format: &ImageFormatExt,
) -> Result<()> {
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();

    let opt = usvg::Options {
        resources_dir: Some(input_path.into()),
        fontdb: fontdb.into(),
        ..Default::default()
    };

    let svg_data = std::fs::read(&input_path)
        .with_context(|| format!("Failed to read file '{input_path:?}'"))?;
    let rtree =
        usvg::Tree::from_data(&svg_data, &opt).with_context(|| "Failed to parse SVG contents")?;

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

    match convert_format {
        ImageFormatExt::Ico => {
            other_to_icon(image.into(), &output_path, vec![16, 32, 48, 64, 128, 256])?
        }
        ImageFormatExt::Jpeg => {
            let image = DynamicImage::ImageRgba8(image).to_rgb8();
            image.save_with_format(
                output_path,
                convert_format
                    .get_format()
                    .expect("No supported image formats"),
            )?
        }
        _ => image.save_with_format(
            output_path,
            convert_format
                .get_format()
                .expect("No supported image formats"),
        )?,
    }
    Ok(())
}

fn other_to_other(
    input_path: &Path,
    output_path: &Path,
    convert_format: &ImageFormatExt,
) -> Result<()> {
    let image = image::open(input_path)?;
    match convert_format.get_format() {
        Some(format) => {
            if format == ImageFormat::Jpeg {
                let image = image.to_rgb8();
                image.save_with_format(output_path, format)?
            } else {
                image.save_with_format(output_path, format)?
            }
        }
        None => {
            if *convert_format == ImageFormatExt::Ico {
                other_to_icon(image, output_path, vec![16, 32, 48, 64, 128, 256])?;
            } else if *convert_format == ImageFormatExt::Svg {
                other_to_svg(input_path, output_path, vtracer::Config::default())?
            }
        }
    }

    Ok(())
}

fn other_to_svg(input_path: &Path, output_path: &Path, config: vtracer::Config) -> Result<()> {
    vtracer::convert_image_to_svg(input_path, output_path, config)
        .map_err(|e| anyhow::anyhow!("Failed to convert to svg: {e}\n{input_path:?}\n"))?;

    Ok(())
}

fn other_to_icon(image: DynamicImage, output_path: &Path, sizes: Vec<u32>) -> Result<()> {
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
