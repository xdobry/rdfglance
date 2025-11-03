use crate::{
    graph_algorithms::{GraphAlgorithm, StatisticValue}, 
    IriIndex,
};

pub type NodePosition = u32;

const IRI_WIDTH: f32 = 300.0;

pub struct StatisticsData {
    // Stores the node iri index and its position in SortedNodeLayout structure that is used for graph algorithms
    pub nodes: Vec<(IriIndex, NodePosition)>,
    pub results: Vec<StatisticsResult>,
    pub pos: f32,
    pub drag_pos: Option<f32>,
    pub column_widths: [f32; 3],
    pub data_epoch: u32,
    pub selected_idx: Option<(IriIndex, usize)>,
}

impl Default for StatisticsData {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            results: Vec::new(),
            pos: 0.0,
            drag_pos: None,
            // Default widths for iri, label, and type
            column_widths: [IRI_WIDTH, 200.0, 200.0],
            data_epoch: 0,
            selected_idx: None,
        }
    }
}
pub struct StatisticsResult {
    values: Vec<f32>,
    statistic_value: StatisticValue,
}

impl StatisticsResult {
    pub fn new_for_alg(values: Vec<f32>, alg: GraphAlgorithm) -> Self {
        Self {
            values,
            statistic_value: alg.get_statistics_values()[0],
        }
    }
    pub fn new_for_values(values: Vec<f32>, statistic_value: StatisticValue) -> Self {
        Self {
            values,
            statistic_value: statistic_value,
        }
    }
    pub fn statistics_value(&self) -> StatisticValue {
        self.statistic_value
    }
    pub fn get_data_vec(&self) -> &Vec<f32> {
        &self.values
    }
    pub fn get_value_str(&self, node_index: usize) -> String {
        let data_vec = self.get_data_vec();
        if node_index < data_vec.len() {
            format!("{:.4}", data_vec[node_index])
        } else {
            "N/A".to_string()
        }
    }
    pub fn swap_values(&mut self, i: usize, j: usize) {
        self.values.swap(i, j);
    }
}

pub fn distribute_to_zoom_layers(values: &Vec<f32>) -> Vec<u8> {
    let mut values_with_indices: Vec<_> = values.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    values_with_indices.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let mut layers = vec![0u8; values.len()];
    let data_len = values.len();
    let a = if data_len < 12 { 1 } else { 4 };
    if let Ok(q) = compute_q(values.len() as f64, a as f64, 10, 1e-10, 1000) {
        let q = if q < 1.0 { 1.0 } else { q };
        let ranges: Vec<(usize, usize)> = {
            let mut ranges = Vec::new();
            let mut pos = 0;
            let mut start = a as f64;
            for idx in 0..10 {
                let end = if idx == 9 {
                    data_len - 1
                } else {
                    (pos as f64 + start + 0.5) as usize - 1
                };
                if end >= data_len - 1 {
                    ranges.push((pos as usize, data_len - 1));
                    break;
                } else {
                    ranges.push((pos as usize, end));
                }
                pos = end + 1;
                start *= q;
            }
            ranges
        };
        let mut corrected_ranges: Vec<(usize, usize)> = Vec::with_capacity(ranges.len());
        let mut next_start: isize = -1;
        for (idx, &(mut start, mut end)) in ranges.iter().enumerate() {
            if next_start >= 0 {
                start = next_start as usize;
            }
            next_start = -1;
            if end < start {
                end = start;
                next_start = (end + 1) as isize;
                if next_start as usize > data_len - 1 {
                    break;
                }
            }
            if idx > 0 {
                let (last_start, mut last_end) = corrected_ranges.last().copied().unwrap();

                // Compare values at the current start and previous range's end
                if values_with_indices[start].0 == values_with_indices[last_end].0 {
                    if values_with_indices[start].0 == values_with_indices[end].0 {
                        if values_with_indices[last_start].0 == values_with_indices[last_end].0 {
                            next_start = (end + 1) as isize;
                            // Extend previous range
                            corrected_ranges.last_mut().unwrap().1 = end;
                            continue;
                        } else {
                            // shrink previous range from the end
                            while values_with_indices[last_end].0 == values_with_indices[start].0 && last_end > last_start {
                                last_end -= 1;
                            }
                            corrected_ranges.last_mut().unwrap().1 = last_end;
                            start = last_end + 1;
                        }
                    } else {
                        // shift start forward to skip duplicates
                        while values_with_indices[last_end].0 == values_with_indices[start].0 && start <= end {
                            start += 1;
                        }
                        corrected_ranges.last_mut().unwrap().1 = start - 1;
                    }
                }
            }
            corrected_ranges.push((start, end));
        }

        for (layer, (start, end)) in corrected_ranges.iter().enumerate() {
            // println!("Layer {}: {} - {}", layer + 1, start, end);
            for (_value, index) in values_with_indices.iter().skip(*start).take(end - start + 1) {
                layers[*index] = 10 - layer as u8;
            }
        }
    }
    layers
}

fn compute_q(sum: f64, a: f64, n: usize, tol: f64, max_iter: usize) -> Result<f64, String> {
    if n == 0 {
        panic!("n must be > 0");
    }
    if (sum - a).abs() < tol {
        return Ok(1.0); // Spezialfall: Summe = erstes Glied -> q=1
    }

    // f(q) = a*(1 - q^n)/(1 - q) - S
    let f = |q: f64| -> f64 {
        if (q - 1.0).abs() < tol {
            // Limes q -> 1
            a * (n as f64) - sum
        } else {
            a * (1.0 - q.powi(n as i32)) / (1.0 - q) - sum
        }
    };

    let mut low = 0.0_f64;
    let mut high = f64::max(2.0, sum / a);

    for _ in 0..max_iter {
        let mid = (low + high) / 2.0;
        let val = f(mid);
        if val.abs() < tol {
            return Ok(mid);
        }
        if f(low) * val < 0.0 {
            high = mid;
        } else {
            low = mid;
        }
    }
    Err("No solution found max_iter reached".to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap};

    use super::*;
    use rand::{seq::SliceRandom, Rng};
    
    fn gen_test_data(desc: &Vec<(u32,f32,f32)>) -> Vec<f32> {
        let mut res = Vec::new();
        let mut start = 1.0;
        for (count ,d,start_diff) in desc {
            start -= start_diff;
            for _ in 0..*count {
                start -= *d;
                res.push(start);
            }
        }
        res
    }

    fn prepare_dist(desc: &Vec<(u32,f32,f32)>) -> (Vec<u8>,BTreeMap<u8,u32>,u8,u8) {
        let test_data = gen_test_data(desc);
        prepare_dist_data(&test_data)
    }

    fn prepare_dist_data(test_data: &Vec<f32>) -> (Vec<u8>,BTreeMap<u8,u32>,u8,u8) {
        let layers = distribute_to_zoom_layers(&test_data);
        assert_eq!(test_data.len(), layers.len());
        let mut max = 0;
        let mut min = 10;
        let mut hist: BTreeMap<u8,u32> = BTreeMap::new();
        for (l,v) in layers.iter().zip(test_data.iter()) {
            // println!("Layer: {} {}", l,v);
            if *l > max {
                max = *l;
            }
            if *l < min {
                min = *l;
            }
            *hist.entry(*l).or_insert(0) += 1;
        }
        (layers, hist, min, max)
    }
    
    #[test]
    fn test_distriute_to_zoo_layers() {
        // cargo test test_distriute_to_zoo_layers -- --nocapture

        let data = vec![
            (10, 0.0, 0.0),
            (2, 0.01, 0.0),
            (3, 0.01, 0.0),
            (4, 0.01, 0.0),
            (5, 0.01, 0.0),
        ];
        let (_layers, hist, min, max) = prepare_dist(&data);
        assert_eq!(max, 10);
        assert_eq!(min, 6);
        assert_eq!(5, hist.len());

        let mut data = Vec::new();
        for _ in 0..1000 {
            data.push((1, 0.0001, 0.0));
        }
        let (layers, hist, min, max) = prepare_dist(&data);
        assert_eq!(max, 10);
        assert_eq!(min, 1);
        assert_eq!(hist.get(&10), Some(&4));
        assert_eq!(10, hist.len());
        layers.windows(2).for_each(|w| {
            assert!(w[0] >= w[1]);
        });

        let data = vec![(5,0.0, 0.0)];
        let (_layers, hist, min, max) = prepare_dist(&data);
        assert_eq!(max, 10);
        assert_eq!(min, 10);
        assert_eq!(1, hist.len());

        let data = vec![(1,0.0, 0.0),(5,0.0, 0.1)];
        let (layers, hist, min, max) = prepare_dist(&data);
        assert_eq!(max, 10);
        assert_eq!(min, 9);
        assert_eq!(2, hist.len());
        assert_eq!(hist.get(&10), Some(&1));
        assert_eq!(hist.get(&9), Some(&5));
        layers.windows(2).for_each(|w| {
            assert!(w[0] >= w[1]);
        });

        let data = vec![(5,0.0, 0.0),(1,0.1, 0.0)];
        let (layers, hist, min, max) = prepare_dist(&data);
        assert_eq!(max, 10);
        assert_eq!(min, 9);
        assert_eq!(2, hist.len());
        assert_eq!(hist.get(&10), Some(&5));
        assert_eq!(hist.get(&9), Some(&1));
        layers.windows(2).for_each(|w| {
            assert!(w[0] >= w[1]);
        });

        let mut data = Vec::new();
        let mut rng = rand::rng();
        for _ in 0..1000 {
            data.push((rng.random_range(1..5), 0.0001, 0.0));
        }
        let mut test_data = gen_test_data(&data);
        test_data.shuffle(&mut rng);
        let (layers, hist, min, max) = prepare_dist_data(&test_data);
        assert_eq!(max, 10);
        assert_eq!(min, 1);
        assert_eq!(10, hist.len());
        let mut test_data_with_index = test_data.iter().enumerate().map(|(i,v)| (v,i)).collect::<Vec<(&f32,usize)>>();
        test_data_with_index.sort_by(|a,b| b.0.partial_cmp(a.0).unwrap());
        let layers_sorted = test_data_with_index.iter().map(|(_v,i)| layers[*i]).collect::<Vec<u8>>();
        layers_sorted.windows(2).for_each(|w| {
            assert!(w[0] >= w[1]);
        });

    }
}