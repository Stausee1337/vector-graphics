use std::ops::Mul;

use crate::vec::Vec2;

#[derive(Clone, Copy, PartialEq)]
pub struct Affine([f32; 6]);

impl Affine {
    pub const IDENTITY: Affine = Affine::scale(1.0);

    pub const fn new(coeffs: [f32; 6]) -> Affine {
        Affine(coeffs)
    }

    pub const fn scale(scale: f32) -> Affine {
        Affine([scale, 0.0, 0.0, scale, 0.0, 0.0])
    }

    pub const fn translate(translation: Vec2) -> Affine {
        Affine([1.0, 0.0, 0.0, 1.0, translation.x, translation.y])
    }

    pub fn rotate(angle: f32) -> Affine {
        let (s, c) = angle.sin_cos();
        Affine([c, -s, s, c, 0.0, 0.0])
    }

    pub fn as_coeffs(self) -> [f32; 6] {
        self.0
    }

    pub fn determinant(self) -> f32 {
        let [a, b, c, d, _, _] = self.0;
        a * d - b * c
    }
}

impl Mul for Affine {
    type Output = Affine;

    fn mul(self, rhs: Self) -> Self::Output {
        // |a b e|   |t u x|
        // |c d f| * |v w y|
        // |0 0 1|   |0 0 1|
        // 
        // |at + bv  au + bw  ax + by + e|
        // |ct + dv  cu + dw  cx + dy + f|
        // |   0        0          1     |

        Self::new([
            self.0[0] * rhs.0[0] + self.0[1] * rhs.0[2],
            self.0[0] * rhs.0[1] + self.0[1] * rhs.0[3],
            self.0[2] * rhs.0[0] + self.0[3] * rhs.0[2],
            self.0[2] * rhs.0[1] + self.0[3] * rhs.0[3],
            self.0[0] * rhs.0[4] + self.0[1] * rhs.0[5] + self.0[4],
            self.0[2] * rhs.0[4] + self.0[3] * rhs.0[5] + self.0[5],
        ])
    }
}

impl Mul<Vec2> for Affine {
    type Output = Vec2;

    fn mul(self, rhs: Vec2) -> Self::Output {
        // |a b e|   |x|
        // |c d f| * |y|
        // |0 0 0|   |1|
        // 
        // |ax + by + e|
        // |cx + dy + f|
        // |     0     |
        Vec2 {
            x: self.0[0] * rhs.x + self.0[1] * rhs.y + self.0[4],
            y: self.0[2] * rhs.x + self.0[3] * rhs.y + self.0[5],
        }
    }
}

