//
// lib.rs
// Copyright (C) 2019 Malcolm Ramsay <malramsay64@gmail.com>
// Distributed under terms of the MIT license.
//
//

#[allow(unused_imports)]
#[macro_use]
extern crate approx;

extern crate nalgebra as na;
extern crate rand;

pub mod basis;
pub mod shape;

use nalgebra::{IsometryMatrix2, Matrix2, Point2, Vector2};
use rand::distributions::{Distribution, Uniform};
use rand::Rng;
use std::f64::consts::PI;

pub use crate::basis::{Basis, SharedValue, StandardBasis};
pub use crate::shape::Shape;

/// The different crystal families that can be represented
///
/// These are all the valid types of crystal symmetries which are valid in a 2D space.
///
#[derive(Debug, Clone, PartialEq)]
pub enum CrystalFamily {
    Monoclinic,
    Orthorhombic,
    Hexagonal,
    Tetragonal,
}

#[cfg(test)]
mod crystal_family_test {
    use super::*;

    #[test]
    fn equality() {
        assert_eq!(CrystalFamily::Monoclinic, CrystalFamily::Monoclinic);
        assert_eq!(CrystalFamily::Orthorhombic, CrystalFamily::Orthorhombic);
        assert_eq!(CrystalFamily::Hexagonal, CrystalFamily::Hexagonal);
        assert_eq!(CrystalFamily::Tetragonal, CrystalFamily::Tetragonal);
    }

    #[test]
    fn inequality() {
        assert_ne!(CrystalFamily::Orthorhombic, CrystalFamily::Monoclinic);
        assert_ne!(CrystalFamily::Hexagonal, CrystalFamily::Monoclinic);
        assert_ne!(CrystalFamily::Tetragonal, CrystalFamily::Monoclinic);
        assert_ne!(CrystalFamily::Hexagonal, CrystalFamily::Orthorhombic);
        assert_ne!(CrystalFamily::Tetragonal, CrystalFamily::Orthorhombic);
        assert_ne!(CrystalFamily::Tetragonal, CrystalFamily::Hexagonal);
    }
}

/// Defining one of the Crystallographic wallpaper groups.
///
/// This is the highest level description of the symmetry operations of a crystal structure.
///
#[derive(Debug, Clone)]
pub struct Wallpaper {
    pub name: String,
    pub family: CrystalFamily,
}

/// Define the transformations of particle positions
///
/// These
#[derive(Debug, Clone)]
pub struct SymmetryTransform {
    isometry: IsometryMatrix2<f64>,
}

impl SymmetryTransform {
    fn parse_ops(ops: &str) -> (Vector2<f64>, f64) {
        let mut vec = Vector2::zeros();
        let mut sign = 1.;
        let mut constant = 0.;
        let mut operator: Option<char> = None;
        for c in ops.chars() {
            match c {
                'x' => {
                    vec[0] = sign;
                    sign = 1.;
                }
                'y' => {
                    vec[1] = sign;
                    sign = 1.;
                }
                '*' | '/' => {
                    operator = Some(c);
                }
                '-' => {
                    sign = -1.;
                }
                '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
                    let val = c.to_string().parse::<u64>().unwrap() as f64;
                    // Is there an operator defined, i.e. is this the first digit
                    if let Some(op) = operator {
                        constant = match op {
                            '/' => sign * constant / val,
                            '*' => sign * constant * val,
                            _ => 0.,
                        };
                        operator = None;
                    } else {
                        constant = sign * val;
                    }
                    sign = 1.
                }
                // Default is do nothing (shouldn't encounter this at all)
                _ => {}
            };
        }

        (vec, constant)
    }

    pub fn new(sym_ops: &str) -> SymmetryTransform {
        let braces: &[_] = &['(', ')'];
        let operations: Vec<&str> = sym_ops
            // Remove braces from front and back
            .trim_matches(braces)
            // Split at the comma
            .split_terminator(',')
            .collect();
        let mut trans = Vector2::new(0., 0.);
        let mut rot: Matrix2<f64> = Matrix2::new(1., 0., 0., 1.);

        for (index, op) in operations.iter().enumerate() {
            let (r, t) = SymmetryTransform::parse_ops(op);
            rot.set_row(index, &r.transpose());
            trans[index] = t;
        }
        SymmetryTransform {
            isometry: IsometryMatrix2::from_parts(
                na::Translation2::from(trans),
                na::Rotation2::from_matrix_unchecked(rot),
            ),
        }
    }

    pub fn transform(&self, position: &Point2<f64>) -> Point2<f64> {
        self.isometry * position
    }

    pub fn rotate(&self, vect: &Vector2<f64>) -> Vector2<f64> {
        self.isometry * vect
    }
}

impl Default for SymmetryTransform {
    fn default() -> Self {
        Self {
            isometry: IsometryMatrix2::identity(),
        }
    }
}

#[cfg(test)]
mod symmetry_transform_tests {
    use super::*;

    fn create_identity() -> SymmetryTransform {
        SymmetryTransform {
            isometry: IsometryMatrix2::identity(),
        }
    }

    #[test]
    fn default() {
        let point = Point2::new(0.2, 0.2);
        let transform = SymmetryTransform::default();
        assert_eq!(transform.transform(&point), point);
    }

    #[test]
    fn identity_transform() {
        let identity = create_identity();
        let point = Point2::new(0.2, 0.2);
        assert_eq!(identity.transform(&point), point);

        let vec = Vector2::new(0.2, 0.2);
        assert_eq!(identity.rotate(&vec), vec);
    }

    #[test]
    fn transform() {
        let isometry = SymmetryTransform {
            isometry: IsometryMatrix2::new(Vector2::new(1., 1.), PI / 2.),
        };

        let point = Point2::new(0.2, 0.2);
        assert_eq!(isometry.transform(&point), Point2::new(0.8, 1.2));

        let vec = Vector2::new(0.2, 0.2);
        assert_eq!(isometry.rotate(&vec), Vector2::new(-0.2, 0.2));
    }

    #[test]
    fn parse_operation_default() {
        let input = String::from("(x, y)");
        let st = SymmetryTransform::new(&input);
        let point = Point2::new(0.1, 0.2);
        assert_relative_eq!(st.transform(&point), Point2::new(0.1, 0.2));
    }

    #[test]
    fn parse_operation_xy() {
        let input = String::from("(-x, x+y)");
        let st = SymmetryTransform::new(&input);
        let point = Point2::new(0.1, 0.2);
        assert_relative_eq!(st.transform(&point), Point2::new(-0.1, 0.3));
    }

    #[test]
    fn parse_operation_consts() {
        let input = String::from("(x+1/2, -y)");
        let st = SymmetryTransform::new(&input);
        let point = Point2::new(0.1, 0.2);
        assert_relative_eq!(st.transform(&point), Point2::new(0.6, -0.2));
    }

    #[test]
    fn parse_operation_neg_consts() {
        let input = String::from("(x-1/2, -y)");
        let st = SymmetryTransform::new(&input);
        let point = Point2::new(0.1, 0.2);
        assert_relative_eq!(st.transform(&point), Point2::new(-0.4, -0.2));
    }

    #[test]
    fn parse_operation_zero_const() {
        let input = String::from("(-y, 0)");
        let st = SymmetryTransform::new(&input);
        let point = Point2::new(0.1, 0.2);
        assert_relative_eq!(st.transform(&point), Point2::new(-0.2, 0.));
    }
}

#[derive(Debug, Clone)]
pub struct WyckoffSite {
    pub letter: char,
    pub symmetries: Vec<SymmetryTransform>,
    pub num_rotations: u64,
    pub mirror_primary: bool,
    pub mirror_secondary: bool,
}

impl WyckoffSite {
    fn multiplicity(&self) -> usize {
        return self.symmetries.len();
    }

    fn degrees_of_freedom(&self) -> &[bool] {
        // TODO implement -> This is only required for the non-general wyckoff sites since all the
        // general sites have 3 degrees-of-freedom.
        //
        // This will be checked as a method of the SymmetryTransform struct.
        return &[true, true, true];
    }
}

#[cfg(test)]
mod wyckoff_site_tests {
    use super::*;

    pub fn create_wyckoff() -> WyckoffSite {
        WyckoffSite {
            letter: 'a',
            symmetries: vec![SymmetryTransform::default()],
            num_rotations: 1,
            mirror_primary: false,
            mirror_secondary: false,
        }
    }

    #[test]
    fn multiplicity() {
        let wyckoff = create_wyckoff();
        assert_eq!(wyckoff.multiplicity(), 1);
    }

}

#[derive(Clone)]
struct OccupiedSite {
    wyckoff: WyckoffSite,
    x: SharedValue,
    y: SharedValue,
    angle: SharedValue,
}

impl OccupiedSite {
    fn multiplicity(&self) -> usize {
        return self.wyckoff.symmetries.len();
    }

    fn from_wyckoff(wyckoff: &WyckoffSite) -> OccupiedSite {
        let x = SharedValue::new(0.);
        let y = SharedValue::new(0.);
        let angle = SharedValue::new(0.);

        return OccupiedSite {
            wyckoff: wyckoff.clone(),
            x,
            y,
            angle,
        };
    }

    fn get_basis(&self, rot_symmetry: u64) -> Vec<StandardBasis> {
        let mut basis: Vec<StandardBasis> = vec![];
        let dof = self.wyckoff.degrees_of_freedom();

        if dof[0] {
            basis.push(StandardBasis::new(&self.x, 0., 1.));
        }
        if dof[1] {
            basis.push(StandardBasis::new(&self.y, 0., 1.));
        }
        if dof[2] {
            basis.push(StandardBasis::new(
                &self.angle,
                0.,
                2. * PI / rot_symmetry as f64,
            ));
        }
        basis
    }
}

#[derive(Clone)]
pub struct Cell {
    x_len: SharedValue,
    y_len: SharedValue,
    angle: SharedValue,
    family: CrystalFamily,
}
impl Cell {
    fn from_family(family: &CrystalFamily, length: f64) -> Cell {
        let (x_len, y_len, angle) = match family {
            // The Hexagonal Crystal has both sides equal with a fixed angle of 60 degrees.
            CrystalFamily::Hexagonal => {
                let len = SharedValue::new(length);
                (len.clone(), len.clone(), SharedValue::new(PI / 3.))
            }
            // The Tetragonal Crystal has both sides equal with a fixed angle of 90 degrees.
            CrystalFamily::Tetragonal => {
                let len = SharedValue::new(length);
                (len.clone(), len.clone(), SharedValue::new(PI / 2.))
            }
            // The Orthorhombic crystal has two variable sides with a fixed angle of 90 degrees.
            CrystalFamily::Orthorhombic => (
                SharedValue::new(length),
                SharedValue::new(length),
                SharedValue::new(PI / 2.),
            ),
            // The Monoclinic cell has two variable sides and a variable angle initialised to 90
            // degrees
            CrystalFamily::Monoclinic => (
                SharedValue::new(length),
                SharedValue::new(length),
                SharedValue::new(PI / 2.),
            ),
        };
        return Cell {
            x_len,
            y_len,
            angle,
            family: family.clone(),
        };
    }

    fn get_basis(&self) -> Vec<StandardBasis> {
        let mut basis: Vec<StandardBasis> = vec![];

        // All cells have at least a single variable cell length
        basis.push(StandardBasis::new(
            &self.x_len,
            0.01,
            self.x_len.get_value(),
        ));

        // Both the Orthorhombic and Monoclinic cells have a second variable cell length
        if (self.family == CrystalFamily::Orthorhombic) | (self.family == CrystalFamily::Monoclinic)
        {
            basis.push(StandardBasis::new(
                &self.y_len,
                0.01,
                self.y_len.get_value(),
            ));
        }

        // The Monoclinic family is the only one to have a variable cell angle.
        if self.family == CrystalFamily::Monoclinic {
            basis.push(StandardBasis::new(&self.angle, PI / 4., 3. * PI / 4.));
        }

        basis
    }

    pub fn area(&self) -> f64 {
        self.angle.get_value().sin() * self.x_len.get_value() * self.y_len.get_value()
    }
}

#[derive(Clone)]
pub struct PackedState {
    pub wallpaper: Wallpaper,
    pub shape: Shape,
    pub cell: Cell,
    occupied_sites: Vec<OccupiedSite>,
    basis: Vec<StandardBasis>,
}

impl PackedState {
    pub fn check_intersection(&self) -> bool {
        // TODO Implement
        return true;
    }

    pub fn total_shapes(&self) -> usize {
        let mut sum: usize = 0;
        for site in self.occupied_sites.iter() {
            sum += site.multiplicity();
        }
        return sum;
    }

    pub fn packing_fraction(&self) -> f64 {
        (self.shape.area() * self.total_shapes() as f64) / self.cell.area()
    }

    pub fn initialise(
        shape: Shape,
        wallpaper: Wallpaper,
        isopointal: Vec<WyckoffSite>,
    ) -> PackedState {
        let mut basis: Vec<StandardBasis> = Vec::new();

        let num_shapes = isopointal.iter().fold(0, |acc, x| acc + x.multiplicity());
        let max_cell_size = 4. * shape.max_radius() * num_shapes as f64;

        let cell = Cell::from_family(&wallpaper.family, max_cell_size);
        basis.append(&mut cell.get_basis());

        let mut occupied_sites: Vec<OccupiedSite> = Vec::new();
        for wyckoff in isopointal.iter() {
            let site = OccupiedSite::from_wyckoff(wyckoff);
            basis.append(&mut site.get_basis(shape.rotational_symmetries));
            occupied_sites.push(site);
        }

        return PackedState {
            wallpaper,
            shape,
            cell,
            occupied_sites,
            basis,
        };
    }
}

#[cfg(test)]
mod packed_state_tests {
    use super::*;

    fn create_square() -> Shape {
        Shape {
            name: String::from("Square"),
            radial_points: vec![1., 1., 1., 1.],
            rotational_symmetries: 4,
            mirrors: 4,
        }
    }

    fn create_wallpaper_p1() -> (Wallpaper, Vec<WyckoffSite>) {
        let wallpaper = Wallpaper {
            name: String::from("p1"),
            family: CrystalFamily::Monoclinic,
        };
        let isopointal = vec![WyckoffSite {
            letter: 'a',
            symmetries: vec![SymmetryTransform::new("x,y")],
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
                SymmetryTransform::new("x,y"),
                SymmetryTransform::new("-x,-y"),
                SymmetryTransform::new("-x+1/2,y"),
                SymmetryTransform::new("x+1/2,-y"),
            ],
            num_rotations: 1,
            mirror_primary: false,
            mirror_secondary: false,
        }];

        (wallpaper, isopointal)
    }

    fn init_packed_state(group: &str) -> PackedState {
        let square = create_square();

        let (wallpaper, isopointal) = (match group {
            "p1" => Some(create_wallpaper_p1()),
            "p2mg" => Some(create_wallpaper_p2mg()),
            _ => None,
        })
        .unwrap();
        PackedState::initialise(square, wallpaper, isopointal)
    }

    #[test]
    fn total_shapes_p1() {
        let state = init_packed_state("p1");
        assert_eq!(state.total_shapes(), 1);
    }

    #[test]
    fn packing_fraction_p1() {
        let state = init_packed_state("p1");
        assert_relative_eq!(state.packing_fraction(), 1. / 8.);
    }

    #[test]
    fn total_shapes_p2mg() {
        let state = init_packed_state("p2mg");
        assert_eq!(state.total_shapes(), 4);
    }

    #[test]
    fn packing_fraction_p2mg() {
        let state = init_packed_state("p2mg");
        assert_relative_eq!(state.packing_fraction(), 1. / 32.);
    }

}

struct MCVars {
    kt_start: f64,
    kt_finish: f64,
    max_step_size: f64,
    num_start_configs: u64,
    steps: u64,
}

impl MCVars {
    fn kt_ratio(&self) -> f64 {
        return f64::powf(self.kt_finish / self.kt_start, 1.0 / self.steps as f64);
    }
}

fn mc_temperature(old: f64, new: f64, kt: f64, n: u64) -> f64 {
    return f64::exp((1. / old - 1. / new) / kt) * (old / new).powi(n as i32);
}

fn monte_carlo_best_packing<'a, 'b>(vars: &'a MCVars, state: &'b mut PackedState) -> PackedState {
    let mut rng = rand::thread_rng();
    let mut rejections: u64 = 0;

    let mut kt: f64 = vars.kt_start;
    let kt_ratio: f64 = vars.kt_ratio();
    let total_shapes: u64 = state.total_shapes() as u64;
    let basis_distribution = Uniform::new(0, state.basis.len() as u64);

    let mut packing: f64 = state.packing_fraction();
    let mut packing_prev: f64 = 0.;
    let mut packing_max: f64 = 0.;

    let mut best_state = state.clone();

    for _ in 0..vars.steps {
        let basis_index: usize = basis_distribution.sample(&mut rng) as usize;
        if let Some(basis_current) = state.basis.get_mut(basis_index) {
            basis_current.set_value(basis_current.sample(&mut rng));
        }

        if state.check_intersection() {
            rejections += 1;
            state.basis[basis_index].reset_value();
        } else {
            packing = state.packing_fraction();
            if rng.gen::<f64>() > mc_temperature(packing_prev, packing, kt, total_shapes) {
                rejections += 1;
                state.basis[basis_index].reset_value();
                packing = packing_prev;
            } else {
                // Keep current state, so update previous packing
                packing_prev = packing;
            }
        }
        if packing > packing_max {
            best_state = state.clone();
            packing_max = packing;
        }
        kt *= kt_ratio;
    }
    println!(
        "Packing Fraction: {}, Rejection Percentage {}",
        packing_max,
        rejections as f64 / vars.steps as f64,
    );
    return best_state;
}
