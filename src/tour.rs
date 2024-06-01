use std::cmp::{max, min};

#[derive(Debug)]
pub struct Tour {
    tour: Vec<usize>,
    inverse: Vec<usize>,
}

impl Tour {
    pub fn new(indexes: Vec<usize>) -> Tour {
        let mut inverse: Vec<usize> = vec![0; indexes.len()];
        for i in 0..indexes.len() {
            let vertex = indexes[i];
            inverse[vertex] = i;
        }
        Tour {
            tour: indexes,
            inverse: inverse,
        }
    }

    pub fn len(&self) -> usize {
        return self.tour.len();
    }

    pub fn flip(&mut self, a: usize, b: usize) {
        assert!(a != b);
        let mut first = min(self.inverse[a], self.inverse[b]);
        let mut second = max(self.inverse[a], self.inverse[b]);
        while first < second {
            let left = self.tour[first];
            let right = self.tour[second];
            self.tour[first] = right;
            self.tour[second] = left;
            self.inverse[right] = first;
            self.inverse[left] = second;

            first += 1;
            second -= 1;
        }
    }

    pub fn next(&self, a: usize) -> usize {
        let idx = self.inverse[a];
        if idx == self.len() - 1 {
            self.tour[0]
        } else {
            self.tour[idx + 1]
        }
    }

    pub fn prev(&self, a: usize) -> usize {
        let idx = self.inverse[a];
        if idx == 0 {
            self.tour[self.len() - 1]
        } else {
            self.tour[idx - 1]
        }
    }

    pub fn simple_between(&self, a: usize, b: usize, candidate: usize) -> bool {
        let left = min(self.inverse[a], self.inverse[b]);
        let right = max(self.inverse[a], self.inverse[b]);
        return self.inverse[candidate] > left && self.inverse[candidate] < right;
    }

    pub fn are_neighbors(&self, a: usize, b: usize) -> bool {
        self.next(a) == b || self.prev(a) == b
    }

    pub fn to_indices(self) -> Vec<usize> {
        return self.tour;
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    use crate::tour::*;
    use crate::testutils::*;

    proptest! {

        #[test]
        fn two_flips_do_nothing(a in 0usize..16, b in 0usize..16) {
            let indices: Vec<usize> = (0..16).collect();
            let original_indices = indices.clone();
            let mut tour = Tour::new(indices);
            if a != b {
            //     let min = min(a, b);
            //     let max = max(a,b);

                tour.flip(a, b);
            //     assert_ne!(original_indices, tour.tour);
            //     if max - 1 > min {
            //         let between = max - 1;
            //         assert!(tour.between(a, b, between));
            //     }
                 tour.flip(a, b);
            //     assert_eq!(original_indices, tour.tour);
            //     if max - 1 > min {
            //         let between = max - 1;
            //         assert!(tour.between(a, b, between));
            //     }
            }
            let flipped_indices = tour.to_indices();
            assert_eq!(original_indices, flipped_indices);
        }

        #[test]
        fn tour_is_tour(length in 2usize..32) {
            let mut indices: Vec<usize> = (0..length).collect();
            let mut rng = thread_rng();
            indices.shuffle(&mut rng);

            let tour = Tour::new(indices);
            let mut assembled = Vec::new();
            let mut vertex = 0;
            for _ in 0..length {
                assembled.push(vertex);
                vertex = tour.next(vertex);
            }
            check_permutation(&assembled, length - 1);
        }
    }

    #[test]
    fn test_flip() {
        let indices = vec![4, 6, 2, 0, 3, 1, 5];
        let mut tour = Tour::new(indices);
        tour.flip(6, 5);
        assert_eq!(tour.to_indices(), vec![4, 5, 1, 3, 0, 2, 6]);
    }

    #[test]
    fn tour_is_tour_2() {
        let length = 6;
        let mut indices: Vec<usize> = (0..length).collect();
        let mut rng = thread_rng();
        indices.shuffle(&mut rng);

        let tour = Tour::new(indices);
        let mut assembled = Vec::new();
        let mut index = 0;
        for _ in 0..length {
            assembled.push(index);
            index = tour.next(index);
        }
        check_permutation(&assembled, length - 1);
    }
}
