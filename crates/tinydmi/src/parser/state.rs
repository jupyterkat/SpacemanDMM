use std::collections::HashMap;

use eyre::{Result, format_err};
use nom::{
    IResult, Parser,
    character::complete::{newline, space1},
    combinator::{map_res, verify},
    multi::many1,
    sequence::{delimited, pair, terminated},
};

use super::{
    key_value::{Dirs, KeyValue, key_value},
    values::Value,
};

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub name: String,
    pub dirs: Dirs,
    pub frames: u32,
    pub delays: Option<Vec<f32>>,
    pub r#loop: bool,
    pub rewind: bool,
    pub movement: bool,
    pub hotspot: Option<[f32; 3]>,
    pub unk: Option<HashMap<String, Value, ahash::RandomState>>,
}

impl State {
    pub fn is_animated(&self) -> bool {
        match self.frames {
            1 => false,
            2.. => true,
            _ => unreachable!(),
        }
    }

    pub fn num_sprites(&self) -> usize {
        let dirs: u32 = self.dirs.into();
        let frames = self.frames;
        (dirs * frames) as usize
    }
}

impl TryFrom<(KeyValue, Vec<KeyValue>)> for State {
    type Error = eyre::Error;

    fn try_from((state, kvs): (KeyValue, Vec<KeyValue>)) -> Result<Self, Self::Error> {
        let name = match state {
            KeyValue::State(name) => name,
            _ => unreachable!(),
        };

        let mut dirs = None;
        let mut frames = 1;
        let mut delays = None;
        let mut r#loop = false;
        let mut rewind = false;
        let mut movement = false;
        let mut hotspot = None;
        let mut unk: Option<HashMap<String, Value, ahash::RandomState>> = None;

        for kv in kvs {
            match kv {
                KeyValue::Dirs(d) => dirs = Some(d),
                KeyValue::Frames(f) => {
                    if frames == 1 {
                        frames = f;
                    } else {
                        return Err(format_err!("Found `frames` in illegal position"));
                    }
                }
                KeyValue::Delay(f) => {
                    if delays.is_none() {
                        delays = Some(f)
                    } else {
                        return Err(format_err!("Found `delay` key without `frames` key"));
                    }
                }
                KeyValue::Loop(do_loop) => r#loop = do_loop,
                KeyValue::Rewind(do_rewind) => rewind = do_rewind,
                KeyValue::Movement(do_movement) => movement = do_movement,
                KeyValue::Hotspot(h) => {
                    if h.len() == 3 {
                        let mut buf = [0.0; 3];
                        buf.copy_from_slice(&h[0..3]);
                        hotspot = Some(buf);
                    } else {
                        return Err(format_err!("Hotspot information was not length 3"));
                    }
                }
                KeyValue::Unk(key, value) => {
                    if let Some(map) = &mut unk {
                        map.insert(key, value);
                    } else {
                        let mut new_map: HashMap<String, Value, ahash::RandomState> =
                            Default::default();
                        new_map.insert(key, value);
                        unk = Some(new_map);
                    }
                }
                x => {
                    return Err(format_err!("{:?} not allowed here", x));
                }
            }
        }

        Ok(State {
            name,
            dirs: dirs.ok_or_else(|| eyre::eyre!("Required field `dirs` was not found"))?,
            frames,
            delays,
            r#loop,
            rewind,
            movement,
            hotspot,
            unk,
        })
    }
}

pub fn state(input: &str) -> IResult<&str, State> {
    map_res(
        pair(
            verify(terminated(key_value, newline), |v| {
                matches!(v, super::key_value::KeyValue::State(_))
            }),
            many1(delimited(space1, key_value, newline)),
        ),
        |(state_name, properties)| State::try_from((state_name, properties)),
    )
    .parse(input)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn metadata() {
        let description = r#"
state = "duplicate"
    dirs = 1
    frames = 1
"#
        .trim();

        let (_, state) = state(description).unwrap();
        assert_eq!(state.dirs, Dirs::One);
        assert_eq!(state.frames, 1);
        assert_eq!(state.name, "duplicate");
    }

    #[test]
    fn delay() {
        let description = r#"
state = "bluespace_coffee"
    dirs = 1
    frames = 4
    delay = 1,2,5.4,3
state = "..."
"#
        .trim();

        let (tail, state) = state(description).unwrap();
        assert_eq!(tail, r#"state = "...""#);
        assert_eq!(state.dirs, Dirs::One);
        assert_eq!(state.delays, Some(Vec::from([1.0, 2.0, 5.4, 3.0])));
        assert_eq!(state.name, "bluespace_coffee");
    }

    #[test]
    fn fail_delay_without_frames() {
        let description = r#"
state = "bluespace_coffee"
    dirs = 1
    delay = 1,2,5.4,3
state = "..."
        "#
        .trim();

        let x = state(description);
        assert!(matches!(x, Err(_)));
    }
}
