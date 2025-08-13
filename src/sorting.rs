/// Sequential allocation-free unstable sort.
/// sports desc on keys
///  Consumes `key` vector to avoid borrow conflicts.
/// `swap` – swaps two row indices in other columns to keep them in sync.
pub fn sort_with_callbacks_keys<Fswap>(sort_values: &mut Vec<f32>, mut swap: Fswap)
where
    Fswap: FnMut(usize, usize),
{
    fn quicksort<Fswap>(key: &mut [f32], lo: usize, hi: usize, swap: &mut Fswap)
    where
        Fswap: FnMut(usize, usize),
    {
        if hi <= lo + 1 {
            return;
        }

        // pivot is last element
        let pivot_index = hi - 1;
        let mut store = lo;
        for i in lo..pivot_index {
            if key[i] > key[pivot_index] {
                key.swap(i, store);
                swap(i, store);
                store += 1;
            }
        }
        key.swap(store, pivot_index);
        swap(store, pivot_index);

        if store > lo {
            quicksort(key, lo, store, swap);
        }
        quicksort(key, store + 1, hi, swap);
    }

    let len = sort_values.len();
    quicksort(sort_values, 0, len, &mut swap);
}

#[cfg(test)]
mod tests {
    use crate::{sorting::{sort_with_callbacks_keys}};

    #[test]
    fn test_sorting() {
        // cargo test test_sorting -- --nocapture
        let mut col1: Vec<u32> = vec![1, 2, 3, 4, 5];
        let mut col2 = vec![0.5, 0.8, 0.3, 1.2, 0.0];
        let mut col3 = vec![0.1, 0.2, 0.3, 0.4, 0.5];

        sort_with_callbacks_keys(
            &mut col2,
            |i, j| {
                col1.swap(i, j);
                col3.swap(i, j);                    
            },
        );

        assert_eq!(col1, vec![5,3,1,2,4]);
        assert_eq!(col2, vec![0.0, 0.3, 0.5, 0.8, 1.2]);
        assert_eq!(col3, vec![0.5, 0.3, 0.1, 0.2, 0.4]);

        println!("{:?}", col1);
        println!("{:?}", col2);
        println!("{:?}", col3);
    }
}
