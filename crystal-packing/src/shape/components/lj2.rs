//
// lj2.rs
// Copyright (C) 2019 Malcolm Ramsay <malramsay64@gmail.com>
// Distributed under terms of the MIT license.
//

use std::fmt;

use nalgebra::Point2;
use serde::{Deserialize, Serialize};

use crate::traits::Potential;

/// A particle which is influences by the Lennard Jones potential
///
/// This defines interactions between particles usign the 12-6 Lennard Jones Potential.
///
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct LJ2 {
    /// The position of a particle within the potential
    pub position: Point2<f64>,
    /// The characteristic distance of the potential
    pub sigma: f64,
    /// The energy of the minimum of the potential
    pub epsilon: f64,
    /// The cutoff for the potential. When this is Some, it indicates the use of the Shifted
    /// Lennard Jones potential.
    pub cutoff: Option<f64>,
}

impl Default for LJ2 {
    fn default() -> LJ2 {
        LJ2 {
            position: Point2::new(0., 0.),
            sigma: 1.,
            epsilon: 1.,
            cutoff: None,
        }
    }
}

impl Potential for LJ2 {
    fn energy(&self, other: &Self) -> f64 {
        let sigma_squared = self.sigma.powi(2);
        let r_squared = (self.position - other.position).norm_squared();
        let sigma2_r2_cubed = (sigma_squared / r_squared).powi(3);

        match self.cutoff {
            Some(x) => {
                if r_squared < x * x {
                    let shift =
                        4. * self.epsilon * ((self.sigma / x).powi(12) - (self.sigma / x).powi(6));
                    4. * self.epsilon * (sigma2_r2_cubed.powi(2) - sigma2_r2_cubed) - shift
                } else {
                    0.
                }
            }
            None => 4. * self.epsilon * (sigma2_r2_cubed.powi(2) - sigma2_r2_cubed),
        }
    }
}

impl fmt::Display for LJ2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "LJ2 {{ {}, {}, {}, {}}}",
            self.position.x, self.position.y, self.sigma, self.epsilon
        )
    }
}

impl LJ2 {
    pub fn new(x: f64, y: f64, sigma: f64) -> Self {
        LJ2 {
            position: Point2::new(x, y),
            sigma,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod test {
    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn init_test() {
        let a = LJ2::new(0., 0., 1.);
        assert_abs_diff_eq!(a.position.x, 0.);
        assert_abs_diff_eq!(a.position.y, 0.);
        assert_abs_diff_eq!(a.sigma, 1.);
        assert_abs_diff_eq!(a.epsilon, 1.);
    }

    #[test]
    fn default_constuctor() {
        let a = LJ2::default();
        assert_abs_diff_eq!(a.position.x, 0.);
        assert_abs_diff_eq!(a.position.y, 0.);
        assert_abs_diff_eq!(a.sigma, 1.);
        assert_abs_diff_eq!(a.epsilon, 1.);
        assert_eq!(a.cutoff, None);
    }

    #[test]
    fn distance_squared_test() {
        let a0 = LJ2::default();
        let a1 = LJ2::new(1., 0., 1.);
        assert_abs_diff_eq!((a0.position - a1.position).norm_squared(), 1.);
        assert_abs_diff_eq!(a0.energy(&a1), 0.);
    }

    #[test]
    fn potential_zero() {
        let a = LJ2::default();
        let b = LJ2::new(1., 0., 1.);
        assert_abs_diff_eq!(a.energy(&b), 0.);
    }

    #[test]
    fn potential_minimum() {
        let a = LJ2::default();
        let b = LJ2::new(2_f64.powf(1. / 6.), 0., 1.);
        dbg!(a.energy(&b));
        assert_abs_diff_eq!(a.energy(&b), -1.);
    }

    #[test]
    fn potential_repulsive() {
        let a = LJ2::default();
        for i in 1..100 {
            let b = LJ2::new(i as f64 / 100., 0., 1.);
            dbg!(a.energy(&b));
            assert!(a.energy(&b) > 0.);
        }
    }

    #[test]
    fn potential_attractive() {
        let a = LJ2::default();
        for i in 1..500 {
            let b = LJ2::new(1. + i as f64 / 100., 0., 1.);
            dbg!(a.energy(&b));
            assert!(-1. < a.energy(&b));
            assert!(a.energy(&b) < 0.);
        }
    }

    #[test]
    fn potential_cutoff() {
        let a = LJ2 {
            position: Point2::new(0., 0.),
            cutoff: Some(3.5),
            ..Default::default()
        };
        let b = LJ2 {
            position: Point2::new(3.5, 0.),
            cutoff: Some(3.5),
            ..Default::default()
        };
        assert_abs_diff_eq!(a.energy(&b), 0.);
    }

    #[test]
    fn potential_cutoff_attractive() {
        let a = LJ2 {
            position: Point2::new(0., 0.),
            cutoff: Some(3.5),
            ..Default::default()
        };
        for i in 1..250 {
            dbg!(i);
            let b = LJ2 {
                position: Point2::new(1. + i as f64 / 100., 0.),
                cutoff: Some(3.5),
                ..Default::default()
            };
            dbg!(a.energy(&b));
            assert!(-1. < a.energy(&b));
            assert!(a.energy(&b) < 0.);
        }
    }
}
