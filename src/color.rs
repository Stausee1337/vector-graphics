
#[derive(Clone, Copy, PartialEq, Eq, bytemuck::Pod)]
#[repr(transparent)]
pub struct Color(u32);

unsafe impl bytemuck::Zeroable for Color { }

impl Color {
    pub const fn new(color: u32) -> Color {
        Color(color)
    }

    pub const fn from_rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Color {
        Color((alpha as u32) << 0o30 | (red as u32) << 0o20 | (green as u32) << 0o10 | blue as u32)
    }

    pub const fn alpha(self) -> u8 {
        (self.0 >> 0o30) as u8
    }

    pub const fn red(self) -> u8 {
        (self.0 >> 0o20) as u8
    }

    pub const fn green(self) -> u8 {
        (self.0 >> 0o10) as u8
    }

    pub const fn blue(self) -> u8 {
        (self.0 >> 0o00) as u8
    }

    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self((self.0 & 0x00FFFFFF) | (alpha as u32) << 0o30)
    }

    pub fn blend(self, other: Color) -> Color {
        let a1 = self.alpha();
        let r1 = self.red()   as u32;
        let g1 = self.green() as u32;
        let b1 = self.blue()  as u32;

        let a2 = other.alpha() as u32;
        let r2 = other.red()   as u32;
        let g2 = other.green() as u32;
        let b2 = other.blue()  as u32;

        let r = ((r1*(255 - a2) + r2*a2)/255).min(255) as u8;
        let g = ((g1*(255 - a2) + g2*a2)/255).min(255) as u8;
        let b = ((b1*(255 - a2) + b2*a2)/255).min(255) as u8;

        Color::from_rgba(r, g, b, a1)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

pub mod colors {
    use super::Color;

    pub const WHITE: Color = Color::new(0xffffffff);
    pub const GRAY : Color = Color::new(0xff7f7f7f);
    pub const BLACK: Color = Color::new(0xff000000);
    pub const RED  : Color = Color::new(0xffff0000);
    pub const LIME : Color = Color::new(0xff00ff00);
    pub const AQUA : Color = Color::new(0xff00ffff);
}
