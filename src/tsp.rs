extern crate bit_set;

use bit_set::BitSet;
use num_traits::int::PrimInt;
use num_traits::bounds::Bounded;

use std::collections::BinaryHeap;
use std::collections::HashSet;

use crate::tour::Tour;

// The LK paper says they only use the 5 nearest nodes
const NEIGHBOR_SIZE: usize = 15;

#[derive(Clone)]
pub struct Tsp<T: PrimInt> {
    pub distances: Vec<T>,
    pub count: usize,
    pub neighbors: Vec<Vec<usize>>,
}

impl<T: PrimInt> Tsp<T> {
    pub fn new(distances: Vec<T>, count: usize) -> Tsp<T> {
        let neighbors = build_neighbors(count, &distances);
        return Tsp {
            distances,
            count,
            neighbors,
        };
    }

    pub fn retrieve_distance(&self, t1: usize, t2: usize) -> u64 {
        return self.distances[distance_index(t1, t2, self.count)]
            .to_u64()
            .unwrap();
    }

    pub fn calculate_distance(&self, indices: &[usize]) -> u64 {
        let mut distance: u64 = 0;
        for i in 0..indices.len() - 1 {
            distance += self.retrieve_distance(indices[i], indices[(i + 1) % self.count]);
        }
        return distance;
    }

    pub fn calculate_distance_tour(&self, tour: &Tour) -> u64 {
        let mut distance: u64 = 0;
        let mut index = 0;
        for _ in 0..tour.len() {
            distance += self.retrieve_distance(index, tour.next(index));
            index = tour.next(index);
        }
        return distance;
    }

    pub fn generate_nearest_neighbor_tour(&self, index: usize) -> Vec<usize> {
        let mut used = HashSet::new();
        let mut tour = vec![index];
        used.insert(index);
        let mut remaining = HashSet::new();
        (0..self.count).for_each(|elem| {
            if elem != index {
                remaining.insert(elem);
            }
        });
        let mut last_index = index;
        while tour.len() < self.count {
            let mut next_index = Option::None;
            for node_distance in self.neighbors[last_index].iter() {
                if used.contains(&node_distance) {
                    continue;
                } else {
                    next_index = Some(*node_distance);
                    break;
                }
            }
            if next_index.is_none() {
                let mut distance = u64::MAX;
                for j in remaining.iter() {
                    if *j == last_index || used.contains(&j) {
                        continue;
                    } else {
                        let edge_distance = self.retrieve_distance(last_index, *j);
                        if edge_distance < distance {
                            distance = edge_distance;
                            next_index = Option::Some(*j);
                        }
                    }
                }
            }
            tour.push(next_index.unwrap());
            used.insert(next_index.unwrap());
            remaining.remove(&next_index.unwrap());
            last_index = next_index.unwrap();
        }
        return tour;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    pub left: usize,
    pub right: usize,
}

impl Edge {
    pub fn new(left: usize, right: usize) -> Edge {
        return Edge {
            left: left,
            right: right,
        };
    }

    pub fn is_neighbor(self: Edge, index: usize) -> bool {
        return self.left == index || self.right == index;
    }
}

// Prim's algorithm
fn minimal_spanning_tree<T: PrimInt + Bounded>(
    count: usize,
    distances: &[T],
    neighbors: &Vec<Vec<usize>>,
) -> Vec<Edge> {
    let mut edges = Vec::new();
    let mut seen = BitSet::with_capacity(count);

    seen.insert(0);

    // Threshold is the longest distance between any node and its closest neighbor--we use this to avoid putting too many edges into the heap
    let mut threshold: T = T::zero();
    for i in 0..neighbors.len() {
        let first_neighbor = neighbors[i][0];
        let distance = distances[distance_index(i, first_neighbor, count)];
        threshold = threshold.max(distance);
    }

    let mut remaining = BinaryHeap::new();
    for i in 1..count {
        let distance = distances[distance_index(0, i, count)];
        remaining.push(EdgeDistance {
            distance: T::max_value() - distance,
            left: 0,
            right: i,
        });
    }

    // We initially only populate the heap with short edges. Only after we've exhausted those do we go back and populate the heap with edges for all remaining vertexes.
    while edges.len() + 1 < count {
        match remaining.pop() {
            Some(next) => {
                if seen.contains(next.left) && seen.contains(next.right) {
                    continue;
                }
                edges.push(Edge {
                    left: next.left,
                    right: next.right,
                });
                let new_index = if seen.contains(next.left) {
                    next.right
                } else {
                    next.left
                };
                seen.insert(new_index);
                for i in 0..count {
                    if new_index != i && !seen.contains(i) {
                        let distance = distances[distance_index(new_index, i, count)];
                        if distance <= threshold {
                            remaining.push(EdgeDistance {
                                distance: T::max_value() - distance,
                                left: new_index,
                                right: i,
                            });
                        }
                    }
                }
            }
            None => {
                for i in 0..count {
                    if !seen.contains(i) {
                        for j in 0..count {
                            if seen.contains(j) {
                                let distance = distances[distance_index(i, j, count)];
                                remaining.push(EdgeDistance {
                                    distance: T::max_value() - distance,
                                    left: i,
                                    right: j,
                                });
                            }
                            threshold = T::max_value();
                        }
                    }
                }
            }
        }
    }
    return edges;
}

fn alpha_neighbors<T: PrimInt + Bounded>(
    count: usize,
    distances: &[T],
    neighbors: &Vec<Vec<usize>>,
) -> Vec<Vec<usize>> {
    let mut alpha_neighbors = Vec::new();
    // need to create minimum 1-tree
    let spanning_tree = minimal_spanning_tree(count, distances, neighbors);
    return alpha_neighbors;
}

fn build_neighbors<T: PrimInt>(count: usize, distances: &[T]) -> Vec<Vec<usize>> {
    let mut neighbors = Vec::new();
    assert!(distances.len() == count * count);
    for i in 0..count {
        let mut heap: BinaryHeap<NodeDistance<T>> = BinaryHeap::new();
        for j in 0..count {
            if i == j {
                continue;
            }
            let distance_index = distance_index(i, j, count);
            if distance_index > distances.len() {
                panic!(
                    "Index {} is out of bounds for length {}",
                    distance_index,
                    distances.len()
                );
            }
            let computed_distance = distances[distance_index];
            if heap.len() < NEIGHBOR_SIZE {
                heap.push(NodeDistance {
                    distance: computed_distance,
                    index: j,
                });
            } else {
                match heap.peek() {
                    Some(NodeDistance { distance, index: _ }) => {
                        if *distance > computed_distance {
                            heap.pop();
                            heap.push(NodeDistance {
                                distance: computed_distance,
                                index: j,
                            });
                        }
                    }
                    None => {
                        heap.push(NodeDistance {
                            distance: computed_distance,
                            index: j,
                        });
                    }
                }
            }
        }
        let mut next_neighbors = Vec::new();
        while heap.peek().is_some() {
            next_neighbors.push(heap.pop().unwrap().index);
        }
        next_neighbors.reverse();
        neighbors.push(next_neighbors);
    }
    neighbors
}

fn distance_index(t1: usize, t2: usize, count: usize) -> usize {
    if t1 < t2 {
        return t1 * count + t2;
    } else {
        return t2 * count + t1;
    }
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
struct NodeDistance<T: PrimInt> {
    distance: T,
    index: usize,
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
struct EdgeDistance<T: PrimInt> {
    distance: T,
    left: usize,
    right: usize,
}

#[cfg(test)]
mod tests {

    use std::collections::HashSet;

    use crate::tsp::{build_neighbors, distance_index, minimal_spanning_tree, Tsp, Edge};
    use crate::testutils::*;

    use proptest::prelude::*;

    #[test]
    fn retrieve_distance_works() {
        let tsp = build_4item_distances();
        for i in 0..4 {
            assert_eq!(0, tsp.retrieve_distance(i, i));
            assert_eq!(255, tsp.retrieve_distance(i, (i + 1) % 4));
            assert_eq!(1, tsp.retrieve_distance(i, (i + 2) % 4));
            assert_eq!(255, tsp.retrieve_distance(i, (i + 3) % 4));
            for j in 0..4 {
                assert_eq!(tsp.retrieve_distance(i, j), tsp.retrieve_distance(j, i));
            }
        }
    }

    #[test]
    fn retrieve_distance_geometric_sample() {
        let distances = build_geo_sample();
        let tsp = Tsp::new(distances, 7);
        for i in 0..7 {
            assert_eq!(0, tsp.retrieve_distance(i, i));
            for j in 0..7 {
                assert_eq!(tsp.retrieve_distance(i, j), tsp.retrieve_distance(j, i));
            }
        }
        assert_eq!(12, tsp.retrieve_distance(0, 2));
        assert_eq!(28, tsp.retrieve_distance(0, 4));
        assert_eq!(29, tsp.retrieve_distance(0, 5));
    }

    #[test]
    fn generate_nearest_neighbor_tour_generates_a_tour() {
        let tsp = build_4item_distances();
        let ordering = tsp.generate_nearest_neighbor_tour(2);
        check_permutation(&ordering, 3);
    }

    #[test]
    fn minimal_spanning_tree_generates_minimal_spanning_tree_example() {
        let count = 7;
        let distances = vec![
            0, 2, 4, 6, 6, 8, 10, 2, 0, 2, 4, 4, 6, 8, 4, 2, 0, 2, 2, 4, 6, 6, 4, 2, 0, 4, 6, 8, 6,
            4, 2, 4, 0, 2, 4, 8, 6, 4, 6, 2, 0, 2, 10, 8, 6, 8, 4, 2, 0,
        ];
        let spanning_tree =
            minimal_spanning_tree(count, &distances, &build_neighbors(count, &distances));
        let mut nodes = HashSet::new();
        for edge in &spanning_tree {
            nodes.insert(edge.left);
            nodes.insert(edge.right);
        }
        for i in 0..count {
            assert!(nodes.contains(&i));
        }
        let mut distance = 0;
        for edge in &spanning_tree {
            distance += distances[distance_index(edge.left, edge.right, count)];
        }
        assert_eq!(12, distance);
    }

    proptest! {
        #[test]
        fn minimal_spanning_tree_creates_a_spanning_tree(v in prop::collection::vec((0u8..=255, 0u8..=255), 2..100)
        .prop_filter("Elements must be distinct", |vec| {
            let set: HashSet<_> = vec.iter().cloned().collect();
            set.len() == vec.len()
        })
    ) {
        let distances: Vec<u16> = build_geometric_distances(&v);
        let tree = minimal_spanning_tree(v.len(), &distances, &build_neighbors(v.len(), &distances));
        check_spanning_tree_is_a_spanning_tree(v.len(), &tree);
    }
    }

    fn check_spanning_tree_is_a_spanning_tree(count: usize, tree: &Vec<Edge>) {
        let mut points = HashSet::new();
        let mut connected = HashSet::new();
        connected.insert(tree[0].left);
        connected.insert(tree[0].right);
        for elem in tree {
            if connected.contains(&elem.left) {
                connected.insert(elem.right);
            }
            points.insert(elem.left);
            points.insert(elem.right);
        }
        assert_eq!(count, points.len());
        assert_eq!(connected, points);
    }

    #[test]
    fn minimal_spanning_tree_generates_minimal_tree_for_geo_sample() {
        let count = 7;
        let distances = build_geo_sample();
        let spanning_tree = minimal_spanning_tree(count, &distances, &build_neighbors(count, &distances));
        let mut distance: u16 = 0;
        for edge in &spanning_tree {
            distance += distances[distance_index(edge.left, edge.right, count)] as u16;
        }
        assert_eq!(51, distance);
    }

    #[test]
    fn build_neighbors_works_for_linear_distances() {
        let distances = build_linear_distances(20);
        let neighbors = build_neighbors(20, &distances);
        let neighbors_0 = &neighbors[0];
        for i in 1..16 {
            assert!(neighbors_0.contains(&i));
        }
        assert_eq!(neighbors_0, &(1..16).collect::<Vec<usize>>());
    }
}
