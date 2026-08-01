
# Vector Graphics

An exploration of concepts and algorithms in the field of CPU vector graphics.

# To Be Done
- [ ] Fix examples
  - [x] Assets directory with
    - [x] JBM
    - [x] Ghostscript Tiger
  - [x] Clean Up winit example
  - [x] Stroking example: kurbo's `tricky_strokes`, if possible, otherwise something simpler
  - [ ] Pico svg parser for ghostscript tiger example
  - [ ] Try using `png` instead of `image` crate for writing pngs
- [ ] Implementation corrections
  - [ ] Drawing transformed strokes correctly
    - [x] Transform path after stroking, not before
    - [ ] Improve quintic root finding in `offset.rs`
    - [ ] Figure out correct, transformed scale for stroking to estimate error tolerance 
        correctly
    - [ ] Resort to drawing hairlines instead of filled primitives when stroke width gets small
  - [ ] Rasterization
    - [x] Some correctness and speedup improvements in `primitives.rs`
    - [ ] Evenodd fill rule
- [ ] Design revisions
  - [ ] Make the Stroker more logical and comprehensible
  - [ ] Rasterization update
    - [ ] Repalce `primitives.rs` with `rasterization.rs` or `raster.rs`
    - [ ] Anti aliased hairline rendering (probably using xiaolin's algorithm)
    - [ ] Gamma correction for anti aliased renders
    - [ ] Better API based on a `DrawTarget` or `Pixmap` and a `Canvas` or `Context`, which 
          orchestrates the drawing
    - [ ] API Objects like `Paint` or `Brush` with different fill styles like `Color`, `Image`
          and `Gradient`
- [ ] Things to add
  - [ ] Documentaion (as we partly have in `path.rs`)
  - [ ] More standard CSS colors (in `color::colors`)
  - [ ] Stroke `Cap`s
  - [ ] Eliptical arcs
    - [ ] Propper `Join::Round` and `Cap::Round` in stroking
  - [ ] Rectangle/Rounded Rectangles
  - [ ] Simple, skrifa based text rendering
  - [ ] Tests
- [ ] Analytical Anti Aliasing
- [ ] Correctness work on kurbo's `tricky_strokes` example
  - [ ] Bounding boxes

