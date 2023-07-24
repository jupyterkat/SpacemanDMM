use ahash::HashMap;
use dmm_tools::dmi::*;
use image::GenericImageView;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

use tinydmi::prelude::{IconLocation, State};

fn is_visible(entry: &DirEntry) -> bool {
    entry
        .path()
        .file_name()
        .unwrap_or("".as_ref())
        .to_str()
        .map(|s| !s.starts_with("."))
        .unwrap_or(true)
}

fn files_with_extension<F: FnMut(&Path)>(ext: &str, mut f: F) {
    let dir = match std::env::var_os("TEST_DME") {
        Some(dme) => Path::new(&dme).parent().unwrap().to_owned(),
        None => {
            println!("Set TEST_DME to check .{} files", ext);
            return;
        }
    };
    for entry in WalkDir::new(dir).into_iter().filter_entry(is_visible) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() && entry.path().extension() == Some(ext.as_ref()) {
            let path = entry.path();
            f(path);
        }
    }
}

fn all_same(icon_file: &IconFile, states: &[(IconLocation, &State)]) -> bool {
    let ((first_index, first_state), rest) = states.split_first().unwrap();
    for (state_index, state) in rest {
        if state.dirs != first_state.dirs || state.frames != first_state.frames {
            return false;
        }
        for i in 0..state.num_sprites() {
            let rect1 = icon_file.get_icon((first_index.into_inner() + i).into());
            let rect2 = icon_file.get_icon((state_index.into_inner() + i).into());

            if rect1
                .pixels()
                .zip(rect2.pixels())
                .find(|((_, _, bit1), (_, _, bit2))| bit1 != bit2)
                .is_some()
            {
                return false;
            }
        }
    }
    true
}

pub fn main() {
    files_with_extension("dmi", |path| {
        let icon_file = IconFile::from_file(path).unwrap();
        let counts = icon_file
            .metadata
            .states
            .iter()
            .map(|(string, state_vec)| {
                let (_, state_sample) = state_vec.get(0).unwrap();
                let new_name = if state_sample.movement {
                    format!("{string} (movement)")
                } else {
                    string.clone()
                };
                let states = state_vec
                    .iter()
                    .map(|(icon_index, state_index)| (*icon_index, state_index))
                    .collect::<Vec<_>>();
                (new_name, states)
            })
            .collect::<HashMap<_, _>>();

        let mut name = false;
        for (k, v) in counts {
            if v.len() > 1 {
                if !name {
                    println!("{}", path.display());
                    name = true;
                }
                let star = if all_same(&icon_file, v.as_slice()) {
                    "*"
                } else {
                    " "
                };
                println!("  {} {}x {}", star, v.len(), k);
            }
        }
    });
}
