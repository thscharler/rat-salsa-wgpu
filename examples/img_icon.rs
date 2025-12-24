use anyhow::{Error, anyhow};
use image::ImageReader;
use std::env::args;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub fn main() -> Result<(), Error> {
    let mut args = args();
    args.next();
    let Some(icon) = args.next() else {
        return Err(anyhow!("img_icon img_file"));
    };

    let path = PathBuf::from(&icon);
    let base_name = path.file_stem().expect("name").to_string_lossy();
    let out_path = path
        .with_file_name(base_name.as_ref())
        .with_extension("raw");
    let out_name = out_path.file_name().expect("name").to_string_lossy();

    let image = ImageReader::open(icon)?;
    let image = image.decode()?;
    let rgba = image.to_rgba8();
    let rgba = rgba.into_flat_samples();

    let (c, w, h) = rgba.extents();

    println!(
        "static IMG: &'static [u8] = include_bytes!({:?});",
        out_name
    );
    println!("Icon::from_rgba(IMG.into(), {}, {});", w, h);

    let mut ff = File::create(out_path)?;
    let layout = rgba.layout;
    let samples = &rgba.samples;
    for y in 0..h {
        for x in 0..w {
            for s in 0..c {
                let off =
                    s * layout.channel_stride + x * layout.width_stride + y * layout.height_stride;
                ff.write(&[samples[off]])?;
            }
        }
    }

    Ok(())
}
