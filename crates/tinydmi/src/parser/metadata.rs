use indexmap::IndexMap;
use std::collections::HashMap;

use eyre::{format_err, Result};
use nom::{
    bytes::complete::tag,
    character::complete::{multispace0, newline, space1},
    combinator::{all_consuming, map_res, verify},
    multi::{many0, many1},
    sequence::{delimited, pair, terminated},
    IResult,
};

use crate::prelude::{Dir, Dirs};

use super::{
    key_value::{key_value, KeyValue},
    state::{state, State},
    values::Value,
};

pub fn begin_dmi(input: &str) -> IResult<&str, &str> {
    terminated(tag("# BEGIN DMI"), newline)(input)
}

pub fn end_dmi(input: &str) -> IResult<&str, &str> {
    terminated(tag("# END DMI"), multispace0)(input)
}

#[derive(Debug)]
pub struct Header {
    pub version: f32,
    pub width: u32,
    pub height: u32,
    pub unk: Option<HashMap<String, Value, ahash::RandomState>>,
}

impl TryFrom<(KeyValue, Vec<KeyValue>)> for Header {
    type Error = eyre::Error;

    fn try_from((state, kvs): (KeyValue, Vec<KeyValue>)) -> Result<Self, Self::Error> {
        let version = match state {
            KeyValue::Version(version) => version,
            _ => unreachable!(),
        };

        if version != 4.0 {
            return Err(format_err!("Version {} not supported, only 4.0", version));
        }

        let mut width = None;
        let mut height = None;
        let mut unk: Option<HashMap<String, Value, ahash::RandomState>> = None;

        for value in kvs {
            match value {
                KeyValue::Width(w) => {
                    width = Some(w);
                }
                KeyValue::Height(h) => {
                    height = Some(h);
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

        Ok(Header {
            version,
            width: width.ok_or_else(|| eyre::eyre!("Required field `width` was not found"))?,
            height: height.ok_or_else(|| eyre::eyre!("Required field `height` was not found"))?,
            unk,
        })
    }
}

pub fn header(input: &str) -> IResult<&str, Header> {
    map_res(
        pair(
            verify(terminated(key_value, newline), |v| {
                matches!(v, KeyValue::Version(_))
            }),
            many1(delimited(space1, key_value, newline)),
        ),
        |(version, properties)| Header::try_from((version, properties)),
    )(input)
}

#[derive(Debug)]
pub struct Metadata {
    pub header: Header,
    pub states: StateMap,
}

impl Metadata {
    pub fn load<S: AsRef<str>>(input: S) -> Result<Metadata> {
        let (_, metadata) = metadata(input.as_ref())
            .map_err(|e| format_err!("Failed to create metadata: {}", e.to_string()))?;

        Ok(metadata)
    }

    pub fn get_icon_state(&self, icon_state: IconIndex<'_>) -> Option<(IconLocation, &State)> {
        let (icon_index, state) = self.states.get(icon_state.1)?.get(icon_state.0)?;
        Some((*icon_index, state))
    }

    pub fn get_duplicate_icon_states(
        &self,
        icon_state: &str,
    ) -> Option<Vec<(IconLocation, &State)>> {
        self.states.get(icon_state).map(|index| {
            index
                .iter()
                .map(|(icon_index, state)| (*icon_index, state))
                .collect::<Vec<_>>()
        })
    }

    pub fn get_index_of_dir(&self, icon_state: IconIndex<'_>, dir: Dir) -> Option<u32> {
        let (first_index, first_state) = self.get_icon_state(icon_state)?;

        let dir_idx = match (first_state.dirs, dir) {
            (Dirs::One, _) => 0,
            (Dirs::Eight, Dir::Northwest) => 7,
            (Dirs::Eight, Dir::Northeast) => 6,
            (Dirs::Eight, Dir::Southwest) => 5,
            (Dirs::Eight, Dir::Southeast) => 4,
            (_, Dir::West) => 3,
            (_, Dir::East) => 2,
            (_, Dir::North) => 1,
            (_, _) => 0,
        };
        Some(first_index.into_inner() as u32 + dir_idx)
    }

    pub fn get_index_of_frame(
        &self,
        icon_state: IconIndex<'_>,
        dir: super::dir::Dir,
        frame: u32,
    ) -> Option<u32> {
        let (first_index, first_state) = self.get_icon_state(icon_state)?;

        let dir_idx = match (first_state.dirs, dir) {
            (Dirs::One, _) => 0,
            (Dirs::Eight, Dir::Northwest) => 7,
            (Dirs::Eight, Dir::Northeast) => 6,
            (Dirs::Eight, Dir::Southwest) => 5,
            (Dirs::Eight, Dir::Southeast) => 4,
            (_, Dir::West) => 3,
            (_, Dir::East) => 2,
            (_, Dir::North) => 1,
            (_, _) => 0,
        };
        Some((first_index.into_inner() as u32 + dir_idx) + frame * first_state.dirs.get_num())
    }
}

pub type StateMap = IndexMap<String, Vec<(IconLocation, State)>, ahash::RandomState>;
// Used to find the actual location on the spritesheet
#[derive(Clone, Copy, Debug)]
pub struct IconLocation(usize);

impl IconLocation {
    pub fn new(num: usize) -> IconLocation {
        IconLocation(num)
    }
    pub fn into_inner(&self) -> usize {
        self.0
    }
}
impl From<usize> for IconLocation {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

// Used to index duplicates
#[derive(Clone, Copy, Debug)]
pub struct IconIndex<'a>(usize, &'a str);

impl<'a> IconIndex<'a> {
    pub fn new(index: usize, icon_name: &'a str) -> IconIndex<'a> {
        IconIndex(index, icon_name)
    }
    pub fn default_icon(icon_name: &'a str) -> IconIndex<'a> {
        IconIndex(0, icon_name)
    }
    pub fn name(&self) -> &'a str {
        self.1
    }
    pub fn index(&self) -> usize {
        self.0
    }
}

impl<'a> From<(usize, &'a str)> for IconIndex<'a> {
    fn from(value: (usize, &'a str)) -> Self {
        Self(value.0, value.1)
    }
}

pub fn metadata(input: &str) -> IResult<&str, Metadata> {
    let (tail, (header, states)) =
        all_consuming(delimited(begin_dmi, pair(header, many0(state)), end_dmi))(input)?;
    let mut state_map: IndexMap<String, Vec<(IconLocation, State)>, ahash::RandomState> =
        Default::default();

    states.into_iter().fold(0, |cursor, state| {
        let num_states = state.frames * state.dirs.get_num();
        state_map
            .entry(state.name.clone())
            .or_default()
            .push((IconLocation(cursor as usize), state));
        cursor + num_states
    });

    Ok((
        tail,
        Metadata {
            header,
            states: state_map,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::parser::key_value::Dirs;

    use super::*;
    #[test]
    fn test_metadata() {
        let description = r#"
# BEGIN DMI
version = 4.0
    width = 32
    height = 32
state = "state1"
    dirs = 4
    frames = 2
    delay = 1.2,1
    movement = 1
    loop = 1
    rewind = 0
    hotspot = 12,13,0
    future = "lmao"
state = "state2"
    dirs = 1
    frames = 1
# END DMI
"#
        .trim();

        let (tail, metadata) = metadata(description).unwrap();
        assert_eq!(tail, "");

        assert_eq!(metadata.header.version, 4.0);
        assert_eq!(metadata.header.width, 32);
        assert_eq!(metadata.header.height, 32);

        assert_eq!(metadata.states[0][0].1.name, "state1".to_string());
        assert_eq!(metadata.states[0][0].1.dirs, Dirs::Four);
        assert_eq!(metadata.states[0][0].1.frames, 2);
        assert_eq!(metadata.states[0][0].1.delays, Some(Vec::from([1.2, 1.0])));
        assert!(metadata.states[0][0].1.movement);
        assert!(metadata.states[0][0].1.r#loop);
        assert!(!metadata.states[0][0].1.rewind);
        assert_eq!(metadata.states[0][0].1.hotspot, Some([12.0, 13.0, 0.0]));

        assert_eq!(metadata.states[1][0].1.name, "state2".to_string());
        assert_eq!(metadata.states[1][0].1.dirs, Dirs::One);
        assert_eq!(metadata.states[1][0].1.frames, 1);

        dbg!(metadata);
    }
}
