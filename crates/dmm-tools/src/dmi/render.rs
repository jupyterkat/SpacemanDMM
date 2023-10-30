use super::*;
use either::Either;
use eyre::Result;
use image::{ImageOutputFormat, RgbaImage};
use tinydmi::prelude::{Dirs, Frames, State};

/// Used to render an IconFile to a .gif/.png easily.
#[derive(Debug)]
pub struct IconRenderer<'a> {
    /// The IconFile we render from.
    source: &'a IconFile,
}

/// [`IconRenderer::render`] will return this to indicate if it wrote to the stream using
/// [`gif::Encoder`] or `[`png::Encoder`].
#[derive(Debug, Clone, Copy)]
pub enum RenderType {
    Png,
    Gif,
}

#[derive(Debug)]
pub struct RenderStateGuard<'a> {
    pub render_type: RenderType,
    renderer: &'a IconRenderer<'a>,
    state: &'a State,
    index: usize,
}

impl<'a> RenderStateGuard<'a> {
    pub fn render<W: std::io::Write + std::io::Seek>(self, target: W) -> Result<()> {
        let icon_index = IconIndex::new(self.index, self.state.name.as_str());
        match self.render_type {
            RenderType::Png => self.renderer.render_to_png(self.state, icon_index, target),
            RenderType::Gif => self.renderer.render_gif(self.state, icon_index, target),
        }
    }
}

/// Public API
impl<'a> IconRenderer<'a> {
    pub fn new(source: &'a IconFile) -> Self {
        Self { source }
    }

    /// Renders with either [`gif::Encoder`] or [`png::Encoder`] depending on whether the icon state is animated
    /// or not.
    /// Returns a [`RenderType`] to help you determine how to treat the written data.
    pub fn prepare_render(&self, icon_state: IconIndex<'_>) -> Result<RenderStateGuard> {
        self.prepare_render_state(
            self.source.get_icon_state(icon_state).ok_or_else(|| {
                eyre::eyre!(
                    "Icon state {}:{} not found!",
                    icon_state.index(),
                    icon_state.name()
                )
            })?,
            icon_state.index(),
        )
    }

    /// This is here so that duplicate icon states can be handled by not relying on the btreemap
    /// of state names in [`Metadata`].
    pub fn prepare_render_state(
        &'a self,
        icon_state: &'a State,
        index: usize,
    ) -> Result<RenderStateGuard> {
        match icon_state.is_animated() {
            false => Ok(RenderStateGuard {
                renderer: self,
                state: icon_state,
                render_type: RenderType::Png,
                index,
            }),
            true => Ok(RenderStateGuard {
                renderer: self,
                state: icon_state,
                render_type: RenderType::Gif,
                index,
            }),
        }
    }

    /// Instead of writing to a file, this gives a Vec<Image> of each frame/dir as it would be composited
    /// for a file.
    pub fn render_to_images(&self, icon_index: IconIndex<'_>) -> Result<Vec<RgbaImage>> {
        let state = self.source.get_icon_state(icon_index).ok_or_else(|| {
            eyre::eyre!(
                "Icon state {}:{} not found!",
                icon_index.index(),
                icon_index.name()
            )
        })?;
        Ok(self.render_frames(state, icon_index))
    }
}

/// Private helpers
impl<'a> IconRenderer<'a> {
    /// Helper for render_to_images- not used for render_gif because it's less efficient.
    fn render_frames(&self, icon_state: &State, icon_index: IconIndex<'_>) -> Vec<RgbaImage> {
        let frames = match &icon_state.frames {
            Frames::One => 1,
            Frames::Count(count) => *count,
            Frames::Delays(delays) => delays.len() as u32,
        };
        let mut canvas = self.get_canvas(icon_state.dirs);
        let mut vec = Vec::new();
        let range = if icon_state.rewind {
            Either::Left((0..frames).chain((0..frames).rev()))
        } else {
            Either::Right(0..frames)
        };
        for frame in range {
            self.render_dirs(icon_state, icon_index, &mut canvas, frame);
            vec.push(canvas.clone());
            canvas
                .pixels_mut()
                .for_each(|pix| *pix = image::Rgba::from([0, 0, 0, 0]));
        }
        vec
    }

    /// Returns a new canvas of the appropriate size
    fn get_canvas(&self, dirs: Dirs) -> RgbaImage {
        match dirs {
            Dirs::One => RgbaImage::new(
                self.source.metadata.header.width,
                self.source.metadata.header.height,
            ),
            Dirs::Four => RgbaImage::new(
                self.source.metadata.header.width * 4,
                self.source.metadata.header.height,
            ),
            Dirs::Eight => RgbaImage::new(
                self.source.metadata.header.width * 8,
                self.source.metadata.header.height,
            ),
        }
    }

    /// Gives a [`Vec<Dir>`] of each [`Dir`] matching our [`Dirs`] setting,
    /// in the same order BYOND uses.
    fn ordered_dirs(dirs: Dirs) -> Vec<Dir> {
        match dirs {
            Dirs::One => [Dir::South].to_vec(),
            Dirs::Four => [Dir::South, Dir::North, Dir::East, Dir::West].to_vec(),
            Dirs::Eight => [
                Dir::South,
                Dir::North,
                Dir::East,
                Dir::West,
                Dir::Southeast,
                Dir::Southwest,
                Dir::Northeast,
                Dir::Northwest,
            ]
            .to_vec(),
        }
    }

    /// Renders each direction to the same canvas, offsetting them to the right
    fn render_dirs(
        &self,
        icon_state: &State,
        icon_index: IconIndex<'_>,
        canvas: &mut RgbaImage,
        frame: u32,
    ) {
        for (dir_no, dir) in Self::ordered_dirs(icon_state.dirs).iter().enumerate() {
            let frame_idx = self
                .source
                .metadata
                .get_index_of_frame(icon_index, *dir, frame)
                .unwrap();
            let frame_rect = self.source.rect_of_index(frame_idx);
            _ = composite(
                &self.source.image,
                canvas,
                (self.source.metadata.header.width * (dir_no as u32), 0),
                frame_rect,
                NO_TINT,
                None,
            );
        }
    }

    /// Renders the whole file to a gif, animated states becoming frames
    fn render_gif<W: std::io::Write + std::io::Seek>(
        &self,
        icon_state: &State,
        icon_index: IconIndex<'_>,
        target: W,
    ) -> Result<()> {
        if !icon_state.is_animated() {
            return Err(eyre::eyre!("Tried to render gif with one frame",));
        }

        let (frames, delays) = match &icon_state.frames {
            Frames::Count(frames) => (*frames, None),
            Frames::Delays(delays) => (delays.len() as u32, Some(delays)),
            _ => unreachable!(),
        };
        let frames = frames as usize;

        let mut canvas = self.get_canvas(icon_state.dirs);

        let mut encoder = image::codecs::gif::GifEncoder::new(target);

        encoder.set_repeat(image::codecs::gif::Repeat::Infinite)?;

        let range = if icon_state.rewind {
            Either::Left((0..frames).chain((0..frames).rev()))
        } else {
            Either::Right(0..frames)
        };

        for frame in range {
            self.render_dirs(icon_state, icon_index, &mut canvas, frame as u32);
            // image::Frame delays are measured in Durations
            let frame = image::Frame::from_parts(
                canvas.clone(),
                0,
                0,
                image::Delay::from_saturating_duration(std::time::Duration::from_secs_f32(
                    delays.map_or_else(|| 1.0, |f| *f.get(frame).unwrap_or(&1.0)) * 0.1, // 1 decisec => 0.1 sec
                )),
            );
            encoder.encode_frame(frame)?;

            canvas
                .pixels_mut()
                .for_each(|pix| *pix = image::Rgba::from([0, 0, 0, 0]));
        }

        Ok(())
    }

    /// Renders the whole file to a png, discarding all but the first frame of animations
    fn render_to_png<W: std::io::Write + std::io::Seek>(
        &self,
        icon_state: &State,
        icon_index: IconIndex<'_>,
        mut target: W,
    ) -> Result<()> {
        let mut canvas = self.get_canvas(icon_state.dirs);

        self.render_dirs(icon_state, icon_index, &mut canvas, 0);

        canvas.write_to(&mut target, ImageOutputFormat::Png)?;
        Ok(())
    }
}
