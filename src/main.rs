//
// main.rs
// Copyright (C) 2019 Malcolm Ramsay <malramsay64@gmail.com>
// Distributed under terms of the MIT license.
//
//

#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate log;
extern crate rayon;

use clap::{App, Arg};
use packing;
use packing::wallpaper::WallpaperGroups;
use rayon::prelude::*;

fn cli() -> clap::ArgMatches<'static> {
    let matches = App::new("packing")
        .version("0.1.0")
        .author("Malcolm Ramsay <malramsay64@gmail.com")
        .about("Find best tilings of 2d shapes")
        .arg(
            Arg::with_name("wallpaper_group")
                .possible_values(&WallpaperGroups::variants())
                .required(true),
        )
        .arg(
            Arg::with_name("sides")
                .long("--num-sides")
                .takes_value(true)
                .default_value("4"),
        )
        .arg(
            Arg::with_name("steps")
                .short("-s")
                .long("--steps")
                .takes_value(true)
                .default_value("100"),
        )
        .get_matches();
    matches
}

fn main() -> Result<(), &'static str> {
    env_logger::init();
    let matches = cli();

    let num_sides: usize = matches.value_of("sides").unwrap().parse().unwrap();
    let polygon = packing::RadialShape {
        name: String::from("Polygon"),
        radial_points: vec![1.; num_sides],
        rotational_symmetries: num_sides as u64,
        mirrors: num_sides as u64,
    };

    let wg = value_t!(matches.value_of("wallpaper_group"), WallpaperGroups).unwrap();

    println!("Using Wallpaper Group: {}", wg);
    let group = packing::wallpaper::get_wallpaper_group(wg).unwrap();

    let wallpaper = packing::Wallpaper::new(&group);
    let isopointal = &[packing::WyckoffSite::new(&group)];

    let state = packing::PackedState::initialise(polygon.clone(), wallpaper.clone(), isopointal);
    if state.check_intersection() {
        panic!("Initial state has intersetions...exiting.");
    }

    println!(
        "Init packing fraction: {}",
        state.packing_fraction().unwrap()
    );

    let mut vars = packing::MCVars::default();
    vars.steps = matches.value_of("steps").unwrap().parse().unwrap();
    vars.num_start_configs = 32;

    let final_state = (0..vars.num_start_configs)
        .into_par_iter()
        .map(|_| {
            let mut state =
                packing::PackedState::initialise(polygon.clone(), wallpaper.clone(), isopointal);
            packing::monte_carlo_best_packing(&vars, &mut state)
        })
        .max()
        .unwrap();

    println!(
        "Cell Area: {}, Shape Area: {}",
        state.cell.area(),
        state.shape.area()
    );
    println!("{:?}", state.cell);

    state.to_figure("test.txt");

    println!(
        "Final packing fraction: {}",
        final_state?.packing_fraction().unwrap()
    );
    Ok(())
}
