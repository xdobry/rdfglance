pub struct Config {
    // nodes force
    pub repulsion_constant: f32,
    // edges force
    pub attraction_factor: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            repulsion_constant: 1.5,
            attraction_factor: 0.05,
        }
    }
}