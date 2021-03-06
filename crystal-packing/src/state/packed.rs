//
// packing.rs
// Copyright (C) 2019 Malcolm Ramsay <malramsay64@gmail.com>
// Distributed under terms of the MIT license.
//

#![allow(clippy::type_repetition_in_bounds)]

use std::cmp::Ordering;
use std::f64::consts::PI;
use std::fmt::Write;
use std::ops::Mul;

use anyhow::Error;
use log::debug;
use serde::{Deserialize, Serialize};

use crate::traits::*;
use crate::wallpaper::{Wallpaper, WallpaperGroup, WyckoffSite};
use crate::{Basis, Cell2, OccupiedSite, Transform2};

pub type PackedState2<S> = PackedState<S>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackedState<S>
where
    S: Shape + Intersect,
{
    pub wallpaper: Wallpaper,
    pub shape: S,
    pub cell: Cell2,
    occupied_sites: Vec<OccupiedSite>,
}

impl<S> Eq for PackedState<S> where S: Shape + Intersect {}

impl<S> PartialEq for PackedState<S>
where
    S: Shape + Intersect,
{
    fn eq(&self, other: &Self) -> bool {
        match (self.score(), other.score()) {
            (Some(s), Some(o)) => s.eq(&o),
            (_, _) => false,
        }
    }
}

impl<S> PartialOrd for PackedState<S>
where
    S: Shape + Intersect,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.score(), other.score()) {
            (Some(s), Some(o)) => s.partial_cmp(&o),
            (_, _) => None,
        }
    }
}

impl<S> Ord for PackedState<S>
where
    S: Shape + Intersect,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<S> State for PackedState<S>
where
    S: Shape + Intersect,
{
    fn total_shapes(&self) -> usize {
        self.occupied_sites
            .iter()
            .fold(0, |sum, site| sum + site.multiplicity())
    }

    fn score(&self) -> Option<f64> {
        if self.check_intersection() {
            None
        } else {
            Some((self.shape.area() * self.total_shapes() as f64) / self.cell.area())
        }
    }

    fn generate_basis(&self) -> Vec<Basis> {
        let mut basis: Vec<Basis> = vec![];
        basis.append(&mut self.cell.get_degrees_of_freedom());
        for site in self.occupied_sites.iter() {
            basis.append(&mut site.get_basis());
        }
        basis
    }

    fn as_positions(&self) -> Result<String, Error> {
        let mut output = String::new();
        writeln!(&mut output, "{}", self.cell)?;
        writeln!(&mut output, "Positions")?;

        for transform in self.cartesian_positions() {
            writeln!(&mut output, "{:?}", transform)?;
        }
        Ok(output)
    }
}
impl<S> PackedState<S>
where
    S: Shape + Intersect,
{
    pub fn cartesian_positions(&self) -> impl Iterator<Item = Transform2> + '_ {
        self.relative_positions()
            .map(move |position| self.cell.to_cartesian_isometry(position))
    }

    pub fn relative_positions(&self) -> impl Iterator<Item = Transform2> + '_ {
        self.occupied_sites.iter().flat_map(OccupiedSite::positions)
    }

    /// Check for intersections of shapes in the current state.
    ///
    /// This checks for intersections between any shapes, checking all occupied sites and their
    /// symmetry defined copies for the current cell and the neighbouring cells. Checking the
    /// neighbouring cells ensures there are no intersections of when tiling space.
    ///
    fn check_intersection(&self) -> bool {
        let periodic_range = match (self.cell.a() / self.cell.b(), self.cell.angle()) {
            (p, a) if 0.5 < p && p < 2. && f64::abs(a - PI / 2.) < 0.2 => 1,
            (p, a) if 0.3 < p && p < 3. && f64::abs(a - PI / 2.) < 0.5 => 2,
            _ => 3,
        };
        // Compare within the current cell
        for (index, shape1) in self
            .cartesian_positions()
            .map(|p| self.shape.transform(&p))
            .enumerate()
        {
            for shape2 in self
                .cartesian_positions()
                .map(|p| self.shape.transform(&p))
                .skip(index + 1)
            {
                if shape1.intersects(&shape2) {
                    return true;
                }
            }
        }

        let radius_sq = self.shape.enclosing_radius().mul(2.).powi(2);
        // Compare in periodic cells
        for transform1 in self.cartesian_positions() {
            let shape1 = self.shape.transform(&transform1);
            for position in self.relative_positions() {
                for transform2 in self.cell.periodic_images(position, periodic_range, false) {
                    let distance = (transform1.position() - transform2.position()).norm_squared();
                    if distance <= radius_sq {
                        let shape2 = self.shape.transform(&transform2);
                        if shape1.intersects(&shape2) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    pub fn initialise(
        shape: S,
        wallpaper: Wallpaper,
        isopointal: &[WyckoffSite],
    ) -> PackedState<S> {
        let num_shapes = isopointal.iter().fold(0, |acc, x| acc + x.multiplicity());
        let max_cell_size = 4. * shape.enclosing_radius() * num_shapes as f64;

        let cell = Cell2::from_family(wallpaper.family, max_cell_size);

        debug!("Cell: {:?}", cell);

        let occupied_sites: Vec<_> = isopointal.iter().map(OccupiedSite::from_wyckoff).collect();

        PackedState {
            wallpaper,
            shape,
            cell,
            occupied_sites,
        }
    }

    pub fn from_group(shape: S, group: &WallpaperGroup) -> Result<Self, Error> {
        let wallpaper = Wallpaper::new(group);
        let isopointal = &[WyckoffSite::new(group)?];
        Ok(Self::initialise(shape, wallpaper, isopointal))
    }
}

#[cfg(test)]
mod packed_state_tests {
    use super::*;
    use crate::{CrystalFamily, LineShape, Transform2};
    use approx::assert_abs_diff_eq;

    fn create_square() -> LineShape {
        LineShape::from_radial("Square", vec![1., 1., 1., 1.]).unwrap()
    }

    fn create_wallpaper_p1() -> (Wallpaper, Vec<WyckoffSite>) {
        let wallpaper = Wallpaper {
            name: String::from("p1"),
            family: CrystalFamily::Monoclinic,
        };
        let isopointal = vec![WyckoffSite {
            letter: 'a',
            symmetries: vec![Transform2::from_operations("x,y").unwrap()],
            num_rotations: 1,
            mirror_primary: false,
            mirror_secondary: false,
        }];

        (wallpaper, isopointal)
    }

    fn create_wallpaper_p2mg() -> (Wallpaper, Vec<WyckoffSite>) {
        let wallpaper = Wallpaper {
            name: String::from("p2mg"),
            family: CrystalFamily::Monoclinic,
        };
        let isopointal = vec![WyckoffSite {
            letter: 'd',
            symmetries: vec![
                Transform2::from_operations("x,y").unwrap(),
                Transform2::from_operations("-x,-y").unwrap(),
                Transform2::from_operations("-x+1/2,y").unwrap(),
                Transform2::from_operations("x+1/2,-y").unwrap(),
            ],
            num_rotations: 1,
            mirror_primary: false,
            mirror_secondary: false,
        }];

        (wallpaper, isopointal)
    }

    fn init_packed_state(group: &str) -> PackedState<LineShape> {
        let square: LineShape = create_square();

        let (wallpaper, isopointal) = (match group {
            "p1" => Some(create_wallpaper_p1()),
            "p2mg" => Some(create_wallpaper_p2mg()),
            _ => None,
        })
        .unwrap();
        PackedState::initialise(square, wallpaper, &isopointal)
    }

    #[test]
    fn total_shapes_p1() {
        let state = init_packed_state("p1");
        assert_eq!(state.total_shapes(), 1);
    }

    #[test]
    fn packing_fraction_p1() {
        let state = init_packed_state("p1");
        assert_abs_diff_eq!(state.score().unwrap(), 1. / 8.);
    }

    #[test]
    fn total_shapes_p2mg() {
        let state = init_packed_state("p2mg");
        assert_eq!(state.total_shapes(), 4);
    }

    #[test]
    fn packing_fraction_p2mg() {
        let state = init_packed_state("p2mg");
        assert_abs_diff_eq!(state.score().unwrap(), 1. / 32.);
    }
}
