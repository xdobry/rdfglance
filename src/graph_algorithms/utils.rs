pub fn normalize(mut values: Vec<f32>) -> Vec<f32> {
    if let Some(&max_val) = values.iter().max_by(|a, b| a.partial_cmp(b).unwrap()) {
        if max_val > 0.0 {
            for v in &mut values {
                *v /= max_val;
            }
        }
    }
    values
}