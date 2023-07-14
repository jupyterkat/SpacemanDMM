use dmm_tools::dmi::*;
use image::GenericImageView;
use std::collections::HashMap;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

use tinydmi::prelude::State;

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

fn all_same(icon_file: &IconFile, states: &[&State]) -> bool {
    let (first, rest) = states.split_first().unwrap();
    let first_start_index = *icon_file.metadata.state_map.get(&first.name).unwrap() as u32;
    for state in rest {
        if state.dirs != first.dirs || state.frames != first.frames {
            return false;
        }
        let start_index = *icon_file.metadata.state_map.get(&state.name).unwrap() as u32;
        for i in 0..state.num_sprites() as u32 {
            let rect1 = icon_file.rect_of_index(first_start_index + i);
            let rect2 = icon_file.rect_of_index(start_index + i);

            let slice1 = icon_file
                .image
                .view(rect1.x, rect1.y, rect1.width, rect1.height);
            let slice2 = icon_file
                .image
                .view(rect2.x, rect2.y, rect2.width, rect2.height);

            if slice1
                .pixels()
                .zip(slice2.pixels())
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
        let mut counts = HashMap::<_, Vec<_>>::new();
        for state in icon_file.metadata.states.iter() {
            let mut name = format!("{:?}", state.name);
            if state.movement {
                name.push_str(" (movement)");
            }
            counts.entry(name).or_default().push(state);
        }
        let mut name = false;
        for (k, v) in counts {
            if v.len() > 1 {
                if !name {
                    println!("{}", path.display());
                    name = true;
                }
                let star = if all_same(&icon_file, &v) { "*" } else { " " };
                println!("  {} {}x {}", star, v.len(), k);
            }
        }
    });
}
