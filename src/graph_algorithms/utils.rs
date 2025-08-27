pub fn normalize(mut values: Vec<f32>) -> Vec<f32> {
    if values.is_empty() {
        return values;
    }

    // one pass to compute (min, max)
    let (mut min_val, mut max_val) = (values[0], values[0]);
    for &v in &values[1..] {
        if v < min_val {
            min_val = v;
        }
        if v > max_val {
            max_val = v;
        }
    }

    let range = max_val - min_val;
    if range > 0.0 {
        for v in &mut values {
            *v = (*v - min_val) / range;
        }
    } else {
        // all values are the same
        for v in &mut values {
            *v = 0.0;
        }
    }

    values
}