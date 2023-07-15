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
    pub states: Vec<State>,
    pub state_map: IndexMap<String, Vec<usize>, ahash::RandomState>,
}

impl Metadata {
    pub fn load<S: AsRef<str>>(input: S) -> Result<Metadata> {
        let (_, metadata) = metadata(input.as_ref())
            .map_err(|e| format_err!("Failed to create metadata: {}", e.to_string()))?;

        Ok(metadata)
    }

    pub fn get_icon_state(&self, icon_state: &str) -> Option<(usize, &State)> {
        let index = *self.state_map.get(icon_state)?.get(0)?;
        Some((index, self.states.get(index)?))
    }

    pub fn get_icon_states(&self, icon_state: &str) -> Option<Vec<(usize, &State)>> {
        self.state_map.get(icon_state).map(|index| {
            index
                .iter()
                .map(|&idx| (idx, self.states.get(idx).unwrap()))
                .collect::<Vec<_>>()
        })
    }

    pub fn get_index_of_dir(&self, icon_state: &str, dir: Dir) -> Option<u32> {
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
        Some(first_index as u32 + dir_idx)
    }

    pub fn get_index_of_frame(
        &self,
        icon_state: &str,
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
        Some((first_index as u32 + dir_idx) + frame * first_state.dirs.get_num())
    }
}

pub fn metadata(input: &str) -> IResult<&str, Metadata> {
    let (tail, (header, states)) =
        all_consuming(delimited(begin_dmi, pair(header, many0(state)), end_dmi))(input)?;
    let mut state_map: IndexMap<String, Vec<usize>, ahash::RandomState> = Default::default();

    let mut cursor = 0;
    for state in states.iter() {
        state_map
            .entry(state.name.clone())
            .or_insert(Vec::new())
            .push(cursor as usize);
        let num_states = state.frames.get_num() * state.dirs.get_num();
        cursor += num_states;
    }
    Ok((
        tail,
        Metadata {
            header,
            states,
            state_map,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::parser::{key_value::Dirs, state::Frames};

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

        assert_eq!(metadata.states[0].name, "state1".to_string());
        assert_eq!(metadata.states[0].dirs, Dirs::Four);
        assert_eq!(
            metadata.states[0].frames,
            Frames::Delays(Vec::from([1.2, 1.0]))
        );
        assert!(metadata.states[0].movement);
        assert!(metadata.states[0].r#loop);
        assert!(!metadata.states[0].rewind);
        assert_eq!(metadata.states[0].hotspot, Some([12.0, 13.0, 0.0]));

        assert_eq!(metadata.states[1].name, "state2".to_string());
        assert_eq!(metadata.states[1].dirs, Dirs::One);
        assert_eq!(metadata.states[1].frames, Frames::One);

        dbg!(metadata);
    }
}
