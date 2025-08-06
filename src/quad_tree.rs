use std::ops::Range;

use egui::{Pos2, Rect, Vec2};

// Source adapted from https://github.com/benbaarber/quadtree/blob/main/src/quadtree/barnes_hut.rs

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct WeightedPoint {
    pub pos: Vec2,
    pub mass: f32,
}

impl WeightedPoint {
    pub fn new(pos: Vec2, mass: f32) -> Self {
        Self { pos, mass }
    }
}


#[derive(Debug)]
struct Node {
    bound: Rect,
    children: usize,
    next: usize,
    cm: WeightedPoint,
    items: Range<usize>,
}

impl Node {
    fn new(bound: Rect, items: Range<usize>, next: usize) -> Self {
        Self {
            bound,
            items,
            next,
            children: 0,
            cm: WeightedPoint::default(),
        }
    }
}

/// A Quadtree specially optimized for the Barnes-Hut algorithm
///
/// An interactive explanation of the algorithm can be found
/// [here](https://jheer.github.io/barnes-hut/)
///
/// This quadtree is immutable and flat, not recursive. It is optimized to be rebuilt frequently
/// and supports efficient accumulation of approximated force. Each step, convert your items into
/// [`WeightedPoint`] and call the build method to clear and reconstruct the tree. Then for each
/// item you need to accumulate force on, call the accumulate method, passing a custom force
/// function.
///
/// This implementation is heavily inspired by [DeadlockCode's Barnes-Hut
/// implementation](https://github.com/DeadlockCode/barnes-hut/tree/improved)
#[derive(Debug)]
pub struct BHQuadtree {
    nodes: Vec<Node>,
    internal_nodes: Vec<usize>,
    items: Vec<WeightedPoint>,
    theta2: f32,
}

impl BHQuadtree {
    /// Create a new empty BHQuadtree with a given theta parameter
    pub fn new(theta: f32) -> Self {
        Self {
            nodes: Vec::new(),
            internal_nodes: Vec::new(),
            items: Vec::new(),
            theta2: theta * theta,
        }
    }

    /// Clear all internal data and reconstruct the tree from a sequence of weighted points
    pub fn build(&mut self, items: Vec<WeightedPoint>, node_capacity: usize) {
        self.nodes.clear();
        self.internal_nodes.clear();
        self.items = items;

        let bound = bound_items(&self.items);
        self.nodes.push(Node::new(bound, 0..self.items.len(), 0));

        let mut n = 0;
        while n < self.nodes.len() {
            let range = self.nodes[n].items.clone();
            if range.len() > node_capacity {
                self.subdivide(n, range);
            } else {
                for i in range {
                    self.nodes[n].cm.pos += self.items[i].pos * self.items[i].mass;
                    self.nodes[n].cm.mass += self.items[i].mass;
                }
            }
            n += 1;
        }

        for &n in self.internal_nodes.iter().rev() {
            let c = self.nodes[n].children;
            for i in 0..4 {
                let cm = self.nodes[c + i].cm;
                self.nodes[n].cm.pos += cm.pos;
                self.nodes[n].cm.mass += cm.mass;
            }
        }

        for node in &mut self.nodes {
            node.cm.pos /= node.cm.mass.max(f32::MIN_POSITIVE);
        }
    }

    /// Accumulate a force vector to act on a target position with an arbitrary force function,
    /// approximating weighted points based on the theta parameter.
    pub fn accumulate<F: Fn(Vec2, WeightedPoint) -> Vec2>(&self, target: Vec2, force_fn: F) -> Vec2 {
        let mut acc = Vec2::ZERO;

        let mut n = 0;
        loop {
            let node = &self.nodes[n];
            let cm = node.cm;
            let d2 = (target-cm.pos).length_sq();
            let s = node.bound.size().max_elem();
            if (s * s) < self.theta2 * d2 {
                acc += force_fn(target, cm);
                n = node.next;
            } else if node.children == 0 {
                for i in node.items.clone() {
                    acc += force_fn(target, self.items[i]);
                }
                n = node.next;
            } else {
                n = node.children;
            }

            if n == 0 {
                break;
            }
        }

        acc
    }

    fn subdivide(&mut self, n: usize, range: Range<usize>) {
        let c = self.nodes.len();
        self.nodes[n].children = c;
        self.internal_nodes.push(n);

        let center = self.nodes[n].bound.center();

        let mut split = [range.start, 0, 0, 0, range.end];

        let predicate = |item: &WeightedPoint| item.pos.y < center.y;
        split[2] = split[0] + self.items[split[0]..split[4]].partition(predicate);

        let predicate = |item: &WeightedPoint| item.pos.x < center.x;
        split[1] = split[0] + self.items[split[0]..split[2]].partition(predicate);
        split[3] = split[2] + self.items[split[2]..split[4]].partition(predicate);

        let bounds = quarter(&self.nodes[n].bound);
        let nexts = [c + 1, c + 2, c + 3, self.nodes[n].next];
        for i in 0..4 {
            let items = split[i]..split[i + 1];
            self.nodes.push(Node::new(bounds[i], items, nexts[i]));
        }
    }
}

pub(crate) trait Partition<T> {
    fn partition<F: Fn(&T) -> bool>(&mut self, predicate: F) -> usize;
}

impl<T> Partition<T> for [T] {
    fn partition<F: Fn(&T) -> bool>(&mut self, predicate: F) -> usize {
        if self.is_empty() {
            return 0;
        }

        let mut l = 0;
        let mut r = self.len() - 1;

        loop {
            while l <= r && predicate(&self[l]) {
                l += 1;
            }
            while l < r && !predicate(&self[r]) {
                r -= 1;
            }
            if l >= r {
                return l;
            }

            self.swap(l, r);
            l += 1;
            r -= 1;
        }
    }
}

pub(crate) fn bound_items(items: &[WeightedPoint]) -> Rect {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for item in items {
        let p = item.pos;
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }

    Rect::from_min_max(Pos2::new(min_x, min_y), Pos2::new(max_x, max_y))
}

pub fn quarter(rect: &Rect) -> [Rect; 4] {
    let center = rect.center();
    let diff = center - rect.min;
    let diff_x = Vec2::new(diff.x, 0.);
    let diff_y = Vec2::new(0., diff.y);

    [
        Rect::from_min_max(rect.min, center),
        Rect::from_min_max(rect.min + diff_x, center + diff_x),
        Rect::from_min_max(rect.min + diff_y, center + diff_y),
        Rect::from_min_max(center, rect.max),
    ]
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_quad_tree() {
        use super::*;
        let mut tree = BHQuadtree::new(0.5);
        let items = vec![
            WeightedPoint::new(Vec2::new(1.0, 1.0), 1.0),
            WeightedPoint::new(Vec2::new(2.0, 2.0), 1.0),
            WeightedPoint::new(Vec2::new(-2.0, -2.0), 1.0),
            WeightedPoint::new(Vec2::new(3.0, 3.0), 1.0),
            WeightedPoint::new(Vec2::new(-5.0, 3.0), 1.0),
        ];
        tree.build(items, 2);

        let targets = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(-2.5, -2.5),
            Vec2::new(3.5, 3.5),
            Vec2::new(20.5, 20.5),
            Vec2::new(1.0, 1.0),
        ];

        let force_fn = |target: Vec2, source: WeightedPoint| {
            // compute repulsive force
            let dir = target - source.pos;
            let dist2 = dir.length_sq().max(1e-4);
            let force_mag = (source.mass) / dist2;
            dir.normalized() * force_mag
        };

        for target in targets {
            let acc = tree.accumulate(target, force_fn);
            let mut acc2 = Vec2::ZERO;
            for item in &tree.items {
                if target == item.pos {
                    continue; // Skip self
                }
                acc2 += force_fn(target, *item);
            }
            assert!((acc-acc2).length()< 0.05); // Allow some tolerance for floating point errors and quad approximation
        }
    }
}