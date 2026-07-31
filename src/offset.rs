use crate::{path::{Cubic, Path}, vec::Vec2};


struct Hermite {
    p0: Vec2,
    v0: Vec2,
    p1: Vec2,
    v1: Vec2
}

impl Hermite {
    fn as_cubic(&self) -> Cubic {
        // H(t)        =        p0 B0(t) +   (p0 + (1/3)v0) B1(t) +   (p1 - (1/3)v1) B2(t) +        p1 B3(t)
        Cubic {
            p0: self.p0,
            p1: self.p0 + self.v0/3.0,
            p2: self.p1 - self.v1/3.0,
            p3: self.p1,
        }
    }
}

pub trait Scalar {
    fn sq(self) -> Self;
}

impl Scalar for f32 {
    fn sq(self) -> Self {
        self * self
    }
}

struct OffsetCubic {
    orig: Cubic,
    approx: Cubic,
    offset: f32,
    coeffs: [f32; 7],
    deriv1_coeffs: [f32; 6],
    deriv2_coeffs: [f32; 5],
}

impl OffsetCubic {
    fn eval_curve_and_derivative(offset: f32, p: Vec2, tan: Vec2, curv: f32) -> (Vec2, Vec2) {
        let point = p + tan.norm().turn90() * offset;
        let deriv = tan * (1.0 - offset * curv);
        (point, deriv)
    }

    fn form_cubic_and_offset(cubic: Cubic, offset: f32) -> Option<Self> {
        // TODO: Potentially do more on numerical hardening instead of exiting out
        let ((tan0, tan1), (curv0, curv1)) = get_cubic_data(&cubic)?;
        let (p0, v0) = Self::eval_curve_and_derivative(offset, cubic.p0, tan0, curv0);
        let (p1, v1) = Self::eval_curve_and_derivative(offset, cubic.p3, tan1, curv1);

        let hermite = Hermite { p0, p1, v0, v1 };

        let i0 = cubic.p0 - hermite.p0;
        let i1 = cubic.p1 - hermite.p0 - hermite.v0/3.0;
        let i2 = cubic.p2 - hermite.p1 + hermite.v1/3.0;
        let i3 = cubic.p3 - hermite.p1;

        let Vec2 { x: x0, y: y0 } = i0;
        let Vec2 { x: x1, y: y1 } = i1;
        let Vec2 { x: x2, y: y2 } = i2;
        let Vec2 { x: x3, y: y3 } = i3;

        let coeffs = [
            x0.sq() + y0.sq(),
            x0*x1 + y0*y1,
            0.4*x0*x2 + 0.6*x1.sq() + 0.4*y0*y2 + 0.6*y1.sq(),
            0.1*x0*x3 + 0.9*x1*x2 + 0.1*y0*y3 + 0.9*y1*y2,
            0.4*x1*x3 + 0.6*x2.sq() + 0.4*y1*y3 + 0.6*y2.sq(),
            x2*x3 + y2*y3,
            x3.sq() + y3.sq(),
        ];
 
        let deriv1_coeffs = [
            coeffs[1] - coeffs[0],
            coeffs[2] - coeffs[1],
            coeffs[3] - coeffs[2],
            coeffs[4] - coeffs[3],
            coeffs[5] - coeffs[4],
            coeffs[6] - coeffs[5]
        ];

        let deriv2_coeffs = [
            deriv1_coeffs[1] - deriv1_coeffs[0],
            deriv1_coeffs[2] - deriv1_coeffs[1],
            deriv1_coeffs[3] - deriv1_coeffs[2],
            deriv1_coeffs[4] - deriv1_coeffs[3],
            deriv1_coeffs[5] - deriv1_coeffs[4],
        ];


        Some(Self {
            orig: cubic,
            approx: hermite.as_cubic(),
            offset,
            coeffs,
            deriv1_coeffs,
            deriv2_coeffs
        })
    }

    fn eval(&self, t: f32) -> f32 {
        let u = 1.0 - t;
        let tt = t.sq();
        let uu = u.sq();

        let b0 = uu*uu*uu;
        let b1 = 6.0*uu*uu*u*t;
        let b2 = 15.0*uu*uu*tt;
        let b3 = 20.0*uu*u*tt*t;
        let b4 = 15.0*uu*tt*tt;
        let b5 = 6.0*u*tt*tt*t;
        let b6 = tt*tt*tt;

        b0 * self.coeffs[0]
            + b1 * self.coeffs[1]
            + b2 * self.coeffs[2]
            + b3 * self.coeffs[3]
            + b4 * self.coeffs[4]
            + b5 * self.coeffs[5]
            + b6 * self.coeffs[6]
    }

    fn eval_deriv1(&self, t: f32) -> f32 {
        let u = 1.0 - t;
        let tt = t.sq();
        let uu = u.sq();

        let b0 = uu*uu*u;
        let b1 = 5.0*uu*uu*t;
        let b2 = 10.0*uu*u*tt;
        let b3 = 10.0*uu*t*tt;
        let b4 = 5.0*u*tt*tt;
        let b5 = tt*tt*t;

        6.0 * (
            b0 * self.deriv1_coeffs[0]
            + b1 * self.deriv1_coeffs[1]
            + b2 * self.deriv1_coeffs[2]
            + b3 * self.deriv1_coeffs[3]
            + b4 * self.deriv1_coeffs[4]
            + b5 * self.deriv1_coeffs[5]
        )
    }

    fn eval_deriv2(&self, t: f32) -> f32 {
        let u = 1.0 - t;
        let tt = t.sq();
        let uu = u.sq();

        let b0 = uu*uu;
        let b1 = 4.0*uu*u*t;
        let b2 = 6.0*uu*tt;
        let b3 = 4.0*u*t*tt;
        let b4 = tt*tt;

        6.0 * 5.0 * (
            b0 * self.deriv2_coeffs[0]
            + b1 * self.deriv2_coeffs[1]
            + b2 * self.deriv2_coeffs[2]
            + b3 * self.deriv2_coeffs[3]
            + b4 * self.deriv2_coeffs[4]
        )
    }

    fn eval_error(&self, t: f32) -> f32 {
        (self.eval(t).abs() - self.offset.sq()).abs()
    }

    fn compute_max_error(&self) -> Option<(f32, f32)> {
        const N_SAMPLES: usize = 51;
        let mut samples: [f32; N_SAMPLES] = [0.0; N_SAMPLES];
        let dt = 1.0 / (N_SAMPLES - 1) as f32;
        for i in 0..N_SAMPLES {
            samples[i] = self.eval_deriv1(dt * i as f32);
        }
    

        let mut i = 0;
        let mut x0s = vec![];
        while i < N_SAMPLES - 1 {
            let s0 = samples[i];
            let s1 = samples[i + 1];
            if s0 * s1 <= 0.0 {
                let t0 = i as f32 * dt;
                x0s.push(t0 + dt*0.5);
            }
            i += 1;
        }

        if x0s.is_empty() { return None; }

        let mut local_max_err = (-1.0, 0.0);
        for x0 in x0s {
            let t_max = self.solve_newton(x0);
            let max = (t_max, self.eval_error(t_max));

            if max.1 > local_max_err.1 {
                local_max_err = max;
            }
        }

        Some(local_max_err)
    }

    fn solve_newton(&self, x0: f32) -> f32 {
        const EPSILON: f32 = 1e-3;
        const MAX_ITER: usize = 4;

        let mut x = x0;
        let mut y;
        let mut y_dot;
        let mut i = 0;
        loop {
            y = self.eval_deriv1(x);
            y_dot = self.eval_deriv2(x);
            x = x - y/y_dot;
            i += 1;

            if y.abs() < EPSILON || i >= MAX_ITER {
                return x;
            }
        }
    }

    fn split(&self, t_subdiv: f32) -> (Option<OffsetCubic>, Option<OffsetCubic>) {
        let (cubic1, cubic2) = self.orig.split_at(t_subdiv);
        // TODO: can probably be optimized by the information we already have
        let oc1 = Self::form_cubic_and_offset(cubic1, self.offset);
        let oc2 = Self::form_cubic_and_offset(cubic2, self.offset);
        (oc1, oc2)
    }
}

struct Approx<'a> {
    dest: &'a mut Path,
}

#[derive(Debug, Clone, Copy)]
pub struct DegenerateCurveError(());

impl<'a> Approx<'a> {
    const MAX_SUBDIV: usize = 10;

    fn approx(dest: &'a mut Path, source: Cubic, offset: f32) -> Result<(), DegenerateCurveError> {
        let Some(oc) = OffsetCubic::form_cubic_and_offset(source, offset) else {
            return Err(DegenerateCurveError(()));
        };
        dest.move_to(oc.approx.p0);
        let mut approx = Approx { dest };

        approx.make_offset_recursively(oc, 0);

        Ok(())
    }

    fn make_offset_recursively(&mut self, oc: OffsetCubic, level: usize) {
        let (t_subdiv, max_err) = self.get_max_error(&oc);
        let approx = &oc.approx;
        if max_err <= 0.25 || level >= Self::MAX_SUBDIV {
            self.dest.curve_to(approx.p1, approx.p2, approx.p3);
            return;
        }
        let next_level = level + 1;
        let (oc1, oc2) = oc.split(t_subdiv);

        // If we get None for oc1 or oc2 it means that we've somehow ended up with a curve with
        // (p0 == p1 == p2 == p3), due to splitting. This really shouldn't be possible so ideally
        // you'd unwrap, but I guess just doing nothing is also fine.
        if let Some(oc1) = oc1 {
            self.make_offset_recursively(oc1, next_level);
        }
        if let Some(oc2) = oc2 {
            self.make_offset_recursively(oc2, next_level);
        }
    }

    fn get_max_error(&self, oc: &OffsetCubic) -> (f32, f32) {
        if let Some((t_subdiv, max_err)) = oc.compute_max_error() {
            return (t_subdiv, max_err.sqrt());
        }
        (0.5, oc.eval_error(0.5).sqrt())
    }
}

fn get_cubic_data(cubic: &Cubic) -> Option<((Vec2, Vec2), (f32, f32))> {
    let (tan0, tan1) = cubic.tangents();
    let (tan0, tan1) = (tan0 * 3.0, tan1 * 3.0);

    const EPSILON: f32 = 1e-6;
    if tan0.length_squared() < EPSILON || tan1.length_squared() < EPSILON {
        // even with all the numerical robustness of Cubic::tangents() it can still be zero 
        // (p0 == p1 == p2 == p3), so a degenerate bezier. In this case we can't build an offset
        // curve.
        return None;
    }

    let derivative1 = cubic.derivative();
    let derivative2 = derivative1.derivative();

    fn get_curvature(d1: Vec2, d2: Vec2) -> f32 {
        // κ = (x'y'' - y'x'')/(x'² + y'²)^(3 / 2)

        let num = d1.x * d2.y - d1.y * d2.x;
        let denom = d1.x.hypot(d1.y);
        num/(denom * denom * denom)
    }

    let curv0 = get_curvature(tan0, derivative2.p0);
    let curv1 = get_curvature(tan1, derivative2.p1);

    Some(((tan0, tan1), (curv0, curv1)))
}

pub fn compute_offset_curve(dest: &mut Path, cubic: Cubic, offset: f32) -> Result<(), DegenerateCurveError> {
    // Peformence and low curve count is hard, and since this is only for reasarch I suppose this approach
    // is fine
    Approx::approx(dest, cubic, offset)
}

