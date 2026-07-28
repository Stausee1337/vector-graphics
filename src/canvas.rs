use crate::color::Color;

pub struct Canvas<'a> {
    pub pixels: &'a mut [u32],
    pub width: u32,
    pub height: u32,
    pub stride: u32
}

impl<'a> Canvas<'a> {
    pub fn from_raw_pixels(pixels: &'a mut [u32], width: usize, height: usize, stride: usize) -> Canvas<'a> {
        assert!(pixels.len() >= width * height);
        Canvas { 
            pixels,
            width: i32::try_from(width).unwrap() as u32,
            height: i32::try_from(height).unwrap() as u32,
            stride: u32::try_from(stride).unwrap()
        }
    }

    pub fn clear(&mut self, color: Color) {
        let (_, height, stride) = (self.width as usize, self.height as usize, self.stride as usize);
        for y in 0..height {
            let line = &mut self.pixels[y * stride..(y + 1)*stride];
            line.fill(color.as_u32());
        }
    }

    pub fn width(&self) -> i32 {
        self.width as i32
    }

    pub fn height(&self) -> i32 {
        self.height as i32
    }

    pub fn at(&self, x: i32, y: i32) -> &Color {
        assert!(x >= 0 && (x as u32) < self.width);
        assert!(y >= 0 && (y as u32) < self.height);
        bytemuck::cast_ref(&self.pixels[y as usize * self.stride as usize + x as usize])
    }

    pub fn at_mut(&mut self, x: i32, y: i32) -> &mut Color {
        assert!(x >= 0 && (x as u32) < self.width);
        assert!(y >= 0 && (y as u32) < self.height);
        bytemuck::cast_mut(&mut self.pixels[y as usize * self.stride as usize + x as usize])
    }
}
