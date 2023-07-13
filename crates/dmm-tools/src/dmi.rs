//! DMI metadata and image composition.
//!
//! Includes re-exports from `dreammaker::dmi`.

use std::{io::Read, path::Path};

use eyre::Result;

use tinydmi::prelude::Dir;

/// Absolute x and y.
pub type Coordinate = (u32, u32);
/// Top left x and y, width and height
pub type Rect = (u32, u32, u32, u32);

// ----------------------------------------------------------------------------
// Icon file and metadata handling
pub mod render;
// Re-exports
pub use tinydmi::prelude::*;

/// An image with associated DMI metadata.
#[derive(Debug)]
pub struct IconFile {
    /// The icon's metadata.
    pub metadata: tinydmi::prelude::Metadata,
    /// The icon's image.
    pub image: image::RgbaImage,
}

impl IconFile {
    pub fn from_file(path: &Path) -> Result<IconFile> {
        let file = std::fs::File::open(path)?;
        let mut reader = std::io::BufReader::new(file);
        let mut buf = vec![];
        reader.read_to_end(&mut buf)?;

        let decoder = png::Decoder::new(buf.as_slice());
        let info = decoder.read_info()?;
        let info = info.info();
        let chunk = info
            .compressed_latin1_text
            .iter()
            .find(|chunk| chunk.keyword == "Description")
            .ok_or_else(|| {
                eyre::eyre!(
                    "Cannot find the description chunk, this might just be a regular png, boss!"
                )
            })?;

        let meta_text = chunk.get_text()?;

        let imagebuf =
            image::io::Reader::with_format(std::io::Cursor::new(buf), image::ImageFormat::Png)
                .decode()?;

        let imagebuf = match imagebuf {
            image::DynamicImage::ImageRgba8(img) => img,
            _ => return Err(eyre::eyre!("Unsupported png type!")),
        };

        Ok(IconFile {
            metadata: tinydmi::parse(meta_text)?,
            image: imagebuf,
        })
    }

    pub fn rect_of(&self, icon_state: &str, dir: Dir) -> Option<Rect> {
        if self.metadata.states.is_empty() {
            return Some((
                0,
                0,
                self.metadata.header.width,
                self.metadata.header.height,
            ));
        }
        let state = self.get_icon_state(icon_state).ok()?;
        let icon_index = state.index_of_frame(dir, 1, &self.metadata.state_map);

        let icon_count = self.image.width() / self.metadata.header.width;
        let (icon_x, icon_y) = (icon_index % icon_count, icon_index / icon_count);
        Some((
            icon_x * self.metadata.header.width,
            icon_y * self.metadata.header.height,
            self.metadata.header.width,
            self.metadata.header.height,
        ))
    }

    pub fn rect_of_index(&self, icon_index: u32) -> Rect {
        let icon_count = self.image.width() / self.metadata.header.width;
        let (icon_x, icon_y) = (icon_index % icon_count, icon_index / icon_count);
        (
            icon_x * self.metadata.header.width,
            icon_y * self.metadata.header.height,
            self.metadata.header.width,
            self.metadata.header.height,
        )
    }

    pub fn get_icon_state(&self, icon_state: &str) -> Result<&tinydmi::prelude::State> {
        self.metadata
            .get_icon_state(icon_state)
            .ok_or_else(|| eyre::eyre!("icon_state {icon_state} not found"))
    }
}

const NO_TINT: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
#[allow(unused)]
const RED: usize = 0;
#[allow(unused)]
const GREEN: usize = 1;
#[allow(unused)]
const BLUE: usize = 2;
const ALPHA: usize = 3;

use image::{GenericImage, GenericImageView};
pub fn composite(
    from: &image::RgbaImage,
    to: &mut image::RgbaImage,
    pos_to: Coordinate,
    crop_from: Rect,
    tint_color: [u8; 4],
) {
    let image_view = from.view(crop_from.0, crop_from.1, crop_from.2, crop_from.3);
    let mut map_view = to.sub_image(pos_to.0, pos_to.1, crop_from.2, crop_from.3);

    image_view
        .pixels()
        .zip(map_view.inner_mut().enumerate_pixels_mut())
        .for_each(|((_, _, from_pix), (_, _, to_pix))| {
            let mut tinted_from = from_pix;

            tinted_from
                .0
                .iter_mut()
                .enumerate()
                .for_each(|(num, channel)| *channel = mul255(*channel, tint_color[num]));
            let out_alpha = tinted_from[ALPHA] + mul255(to_pix[ALPHA], 255 - tinted_from[ALPHA]);

            if out_alpha != 0 {
                (0..3).for_each(|i| {
                    to_pix[i] = (tinted_from[i] * tinted_from[ALPHA]
                        + to_pix[i] * to_pix[ALPHA] * (255 - tinted_from[ALPHA]) / 255)
                        / out_alpha;
                })
            } else {
                (0..3).for_each(|i| to_pix[i] = 0)
            }
            to_pix[ALPHA] = out_alpha;
        });

    #[inline]
    fn mul255(x: u8, y: u8) -> u8 {
        (x as u16 * y as u16 / 255) as u8
    }
}
/*
// ----------------------------------------------------------------------------
// Image manipulation

/// A two-dimensional RGBA image.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Array2<Rgba8>,
}

impl Image {
    pub fn new_rgba(width: u32, height: u32) -> Image {
        Image {
            width,
            height,
            data: { Array2::default((width as usize, height as usize)) },
        }
    }

    fn from_rgba(bitmap: lodepng::Bitmap<RGBA>) -> Image {
        Image {
            width: bitmap.width as u32,
            height: bitmap.height as u32,
            data: {
                let cast_input = bytemuck::cast_slice(bitmap.buffer.as_slice());
                let mut arr = Array2::default((bitmap.width, bitmap.height));
                arr.as_slice_mut().unwrap().copy_from_slice(cast_input);
                arr
            },
        }
    }

    /// Read an `Image` from a [u8] array.
    ///
    /// Prefer to call `IconFile::from_bytes`, which can read both metadata and
    /// image contents at one time.
    pub fn from_bytes(data: &[u8]) -> Result<Image> {
        let mut decoder = Decoder::new();
        decoder.info_raw_mut().colortype = ColorType::RGBA;
        decoder.info_raw_mut().set_bitdepth(8);
        decoder.read_text_chunks(false);
        decoder.remember_unknown_chunks(false);
        let bitmap = match decoder.decode(data) {
            Ok(::lodepng::Image::RGBA(bitmap)) => bitmap,
            Ok(_) => return Err(io::Error::new(io::ErrorKind::InvalidData, "not RGBA")),
            Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e)),
        };

        Ok(Image::from_rgba(bitmap))
    }

    /// Read an `Image` from a file.
    ///
    /// Prefer to call `IconFile::from_file`, which can read both metadata and
    /// image contents at one time.
    pub fn from_file(path: &Path) -> Result<Image> {
        let path = &dm::fix_case(path);
        Self::from_bytes(&std::fs::read(path)?)
    }

    pub fn clear(&mut self) {
        self.data.fill(Default::default())
    }

    #[cfg(feature = "png")]
    pub fn to_write<W: std::io::Write>(&self, writer: W) -> Result<()> {
        {
            let mut encoder = png::Encoder::new(writer, self.width, self.height);
            encoder.set_color(::png::ColorType::Rgba);
            encoder.set_depth(::png::BitDepth::Eight);
            let mut writer = encoder.write_header()?;
            // TODO: metadata with write_chunk()
            writer.write_image_data(bytemuck::cast_slice(self.data.as_slice().unwrap()))?;
        }
        Ok(())
    }

    #[cfg(feature = "png")]
    pub fn to_file(&self, path: &Path) -> Result<()> {
        self.to_write(std::fs::File::create(path)?)
    }

    #[cfg(feature = "png")]
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut vector = Vec::new();
        self.to_write(&mut vector)?;
        Ok(vector)
    }

    pub fn composite(&mut self, other: &Image, pos: Coordinate, crop: Rect, color: [u8; 4]) {
        let other_dat = other.data.as_slice().unwrap();
        let self_dat = self.data.as_slice_mut().unwrap();
        let mut sy = crop.1;
        for y in pos.1..(pos.1 + crop.3) {
            let mut sx = crop.0;
            for x in pos.0..(pos.0 + crop.2) {
                let src = other_dat[(sy * other.width + sx) as usize];
                macro_rules! tint {
                    ($i:expr) => {
                        mul255(src[$i], color[$i])
                    };
                }
                let dst = &mut self_dat[(y * self.width + x) as usize];
                let src_tint = Rgba8::new(tint!(0), tint!(1), tint!(2), tint!(3));

                // out_A = src_A + dst_A (1 - src_A)
                // out_RGB = (src_RGB src_A + dst_RGB dst_A (1 - src_A)) / out_A
                let out_a = src_tint.a + mul255(dst.a, 255 - src_tint.a);
                if out_a != 0 {
                    for i in 0..3 {
                        dst[i] = ((src_tint[i] as u32 * src_tint.a as u32
                            + dst[i] as u32 * dst.a as u32 * (255 - src_tint.a as u32) / 255)
                            / out_a as u32) as u8;
                    }
                } else {
                    for i in 0..3 {
                        dst[i] = 0;
                    }
                }
                dst.a = out_a;

                sx += 1;
            }

            sy += 1;
        }
    }
}

#[inline]
fn mul255(x: u8, y: u8) -> u8 {
    (x as u16 * y as u16 / 255) as u8
}
*/
