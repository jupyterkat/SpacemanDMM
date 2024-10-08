//! DMI metadata and image composition.
//!
//! Includes re-exports from `dreammaker::dmi`.

use std::{io::Read, path::Path};

use eyre::Result;

/// Absolute x and y.
pub type Coordinate = (u32, u32);

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    //top left x
    pub x: u32,
    //top left y
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn bottom_right_x(&self) -> u32 {
        self.x + self.width
    }
    pub fn bottom_right_y(&self) -> u32 {
        self.y + self.height
    }
}

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
        IconFile::from_bytes(buf.as_slice())
    }

    pub fn from_bytes(buf: &[u8]) -> Result<IconFile> {
        let decoder = png::Decoder::new(buf);
        let mut reader = decoder.read_info()?;

        //We only read one frame because dmis should only have one frame.
        let mut image: Vec<u8> = vec![0; reader.output_buffer_size()];
        reader.next_frame(&mut image)?;
        reader.finish()?;

        let chunk = reader
            .info()
            .compressed_latin1_text
            .iter()
            .find(|chunk| chunk.keyword == "Description")
            .ok_or_else(|| {
                eyre::eyre!(
                    "Cannot find the description chunk, make sure that a proper zTXT chunk exists, and is placed before the IDAT chunks!"
                )
            })?;

        let meta_text = chunk.get_text()?;

        //let image = handle_image_colors(reader, image)?;
        let image =
            image::ImageReader::with_format(std::io::Cursor::new(buf), image::ImageFormat::Png)
                .decode()?;
        //it has to be a rgba8 image
        let image = image.to_rgba8();

        Ok(Self {
            metadata: tinydmi::parse(meta_text)?,
            image,
        })
    }

    pub fn get_icon(&self, index: IconLocation) -> image::SubImage<&image::RgbaImage> {
        let icon_count = self.image.width() / self.metadata.header.width;
        let (icon_x, icon_y) = (
            index.into_inner() as u32 % icon_count,
            index.into_inner() as u32 / icon_count,
        );

        self.image.view(
            icon_x,
            icon_y,
            self.metadata.header.width,
            self.metadata.header.height,
        )
    }

    pub fn rect_of(&self, icon_state: IconIndex<'_>, dir: Dir) -> Option<Rect> {
        if self.metadata.states.is_empty() {
            return Some(Rect {
                x: 0,
                y: 0,
                width: self.metadata.header.width,
                height: self.metadata.header.height,
            });
        }
        let icon_index = self.metadata.get_index_of_frame(icon_state, dir, 0)?;
        Some(self.rect_of_index(icon_index))
    }

    pub fn rect_of_index(&self, icon_index: u32) -> Rect {
        let icon_count = self.image.width() / self.metadata.header.width;
        let (icon_x, icon_y) = (icon_index % icon_count, icon_index / icon_count);
        Rect {
            x: icon_x * self.metadata.header.width,
            y: icon_y * self.metadata.header.height,
            width: self.metadata.header.width,
            height: self.metadata.header.height,
        }
    }

    pub fn get_icon_state(&self, icon_state: IconIndex<'_>) -> Option<&tinydmi::prelude::State> {
        self.metadata
            .get_icon_state(icon_state)
            .map(|(_, state)| state)
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
    to_loc: (u32, u32),
    crop_from: Rect,
    tint_color: [u8; 4],
    //transform: Option<[f32; 6]>,
) -> Result<()> {
    if crop_from.x + crop_from.width > from.width()
        || crop_from.y + crop_from.height > from.height()
    {
        return Err(eyre::eyre!(
            "Cannot get subview, out of bounds! {crop_from:?}, (img_width, img_height) {}:{}",
            from.width(),
            from.height()
        ));
    }
    let mut image_copy = from
        .view(crop_from.x, crop_from.y, crop_from.width, crop_from.height)
        .to_image();

    // if let Some(thin) = transform {
    //     if let Some(projection) = imageproc::geometric_transformations::Projection::from_matrix([
    //         1.0 + thin[0],
    //         thin[1],
    //         thin[2],
    //         thin[3],
    //         1.0 + thin[4],
    //         thin[5],
    //         0.0,
    //         0.0,
    //         1.0,
    //     ]) {
    //         image_copy = imageproc::geometric_transformations::warp(
    //             &image_copy,
    //             &projection,
    //             imageproc::geometric_transformations::Interpolation::Nearest,
    //             image::Rgba::from([0, 0, 0, 0]),
    //         )
    //     }
    // }
    let mut new_to_pix = image::ImageBuffer::new(crop_from.width, crop_from.height);

    image_copy
        .pixels_mut()
        .zip(
            to.sub_image(to_loc.0, to_loc.1, crop_from.width, crop_from.height)
                .pixels(),
        )
        .zip(new_to_pix.pixels_mut())
        .for_each(|((from_pix, (_, _, mut to_pix)), to_pix_new)| {
            //tint
            from_pix
                .0
                .iter_mut()
                .zip(tint_color.iter())
                .for_each(|(channel, tint_channel)| *channel = mul255(*channel, *tint_channel));
            let out_alpha = from_pix[ALPHA] + mul255(to_pix[ALPHA], 255 - from_pix[ALPHA]);
            let from_alpha = from_pix[ALPHA];
            let to_alpha = to_pix[ALPHA];

            if out_alpha == 0 {
                *to_pix_new = to_pix;
                return;
            }

            //actual blend
            to_pix
                .0
                .iter_mut()
                .zip(from_pix.0.iter())
                .take(3)
                .for_each(|(to, &from)| {
                    *to = ((from as u32 * from_alpha as u32
                        + *to as u32 * to_alpha as u32 * (255 - from_alpha as u32) / 255)
                        / out_alpha as u32) as u8;
                });
            to_pix[ALPHA] = out_alpha;
            *to_pix_new = to_pix;
        });

    to.sub_image(to_loc.0, to_loc.1, crop_from.width, crop_from.height)
        .copy_from(&new_to_pix, 0, 0)
        .unwrap();

    #[inline]
    fn mul255(x: u8, y: u8) -> u8 {
        (x as u32 * y as u32 / 255) as u8
    }
    Ok(())
}

#[test]
fn composite_test() {
    let mut map = image::RgbaImage::new(4, 4);
    let mut image = image::RgbaImage::new(4, 4);

    image
        .pixels_mut()
        .for_each(|rgba| *rgba = [255, 0, 0, 255].into());

    composite(
        &image,
        &mut map,
        (0, 0),
        Rect {
            x: 0,
            y: 0,
            width: 2,
            height: 2,
        },
        NO_TINT,
        //None,
    )
    .unwrap();

    let map_vec = map.view(0, 0, 2, 2).pixels().collect::<Vec<_>>();
    let image_vec = image.view(0, 0, 2, 2).pixels().collect::<Vec<_>>();
    let map = map
        .enumerate_pixels()
        .map(|(x, y, img)| (x, y, *img))
        .collect::<Vec<_>>();
    let image = image
        .enumerate_pixels()
        .map(|(x, y, img)| (x, y, *img))
        .collect::<Vec<_>>();

    println!("{map_vec:?}");
    println!("{image_vec:?}");

    println!("--------");
    println!("{map:?}");
    println!("{image:?}");
    assert!(map_vec == image_vec)
}
