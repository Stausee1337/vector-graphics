
# Vector Graphics

An exploration of concepts and alogrithms in the field of CPU vector graphics.

# Roadmap ("Nonlinear")
 [ ] Fix examples
   [ ] Assets directory with
     [ ] JBM
     [ ] Ghostscript Tiger
   [ ] Clean Up winit example
   [ ] Stroking example: kurbo's `tricky_strokes`, if possible, otherwise something simpler
   [ ] Pico svg parser for ghostscript exampleghostscript example
   [ ] Try using `png` instead of `image` crate for writing pngs
 [ ] Implementation corrections
   [ ] Drawing transfomed strokes correctly
     [ ] Tansform path after stroking, not before
     [ ] Figure out correct, tansformed scale for stroking to estimate error tolerance correctly
     [ ] Resort to drawing hairlines instead of filled primitives when stroke width gets to small
   [ ] Rasterization
     [ ] Some correctness and speedup improvements in `primitives.rs`
     [ ] Evenodd fill rule
 [ ] Design revisions
   [ ] Make the Stroker more logical and comprehensible
   [ ] Rasterization update
     [ ] Repalce `primitives.rs` with `rasterization.rs` or `scan.rs`
     [ ] Anti aliased hairline rendering (probably using xiaolin's algorithm)
     [ ] Gamma correction for anti aliased renders
     [ ] Better API based on a `DrawTarget` or `Pixmap` and a `Canvas` or `Context`, which 
       orchestrates the drawing
     [ ] API Objects like `Paint` or `Brush` with different fill styles like `Color`, `Image` and 
       `Gradient`
 [ ] Things to add
    [-] Documentaion (as we partly have in `path.rs`)
    [ ] More standard CSS colors (in `color::colors`)
    [ ] Eliptical arcs
      [ ] Propper `Join::Round` and `Cap::Round` in stroking
    [ ] Rectangle/Rounded Rectangles
    [ ] Simple, skrifa based text rendering
 [ ] Coverage based anti aliasing

