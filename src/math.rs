pub fn trunc_falloff(mut x: f32, m: f32) -> f32 {
    x /= m;
    (x - 2.0) * x + 1.0
}
