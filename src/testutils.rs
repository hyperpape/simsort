use std::collections::HashSet;

use num_traits::PrimInt;
use rand::Rng;
use std::fs::read_to_string;
use std::path::PathBuf;

use crate::tsp::Tsp;

#[cfg(test)]
pub fn build_linear_distances(file_count: usize) -> Vec<u8> {
    let mut distances = vec![0; file_count * file_count];
    for i in 0..file_count {
        for j in 0..file_count {
            distances[i * file_count + j] = (i as i64 - j as i64).abs() as u8;
        }
    }
    distances
}

#[cfg(test)]
pub fn build_distances(file_count: usize) -> Vec<u8> {
    let mut distances = vec![0; file_count * file_count];
    for i in 0..file_count {
        for j in 0..file_count {
            distances[i * file_count + j] = ((i % 4) as i64 - (j % 4) as i64).abs() as u8;
        }
    }
    distances
}

#[cfg(test)]
pub fn build_random_distances(file_count: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let mut distances = vec![0; file_count * file_count];
    for i in 0..file_count {
        for j in 0..file_count {
            if i <= j {
                distances[i * file_count + j] = rng.gen_range(0..255);
            } else {
                distances[i * file_count + j] = distances[j * file_count + i];
            }
        }
    }
    distances
}

#[cfg(test)]
pub fn build_geometric_distances<T: PrimInt>(points: &Vec<(u8, u8)>) -> Vec<T> {
    let mut distances: Vec<T> = vec![T::from(0).unwrap(); points.len() * points.len()];
    for i in 0..points.len() {
        for j in 0..points.len() {
            let x_diff = (points[i].0 as i64 - points[j].0 as i64).abs();
            let y_diff = (points[i].1 as i64 - points[j].1 as i64).abs();
            let distance = ((x_diff * x_diff + y_diff * y_diff) as f64).sqrt();
            distances[i * points.len() + j] = T::from(distance).unwrap();
        }
    }
    return distances;
}

#[cfg(test)]
pub fn build_geometric_distances_alt<T: PrimInt>(points: Vec<(f32, f32)>) -> Vec<T> {
    let mut distances: Vec<T> = vec![T::from(0).unwrap(); points.len() * points.len()];
    for i in 0..points.len() {
        for j in 0..points.len() {
            let x_diff = (points[i].0 - points[j].0).abs();
            let y_diff = (points[i].1 - points[j].1).abs();
            let distance = ((x_diff * x_diff + y_diff * y_diff) as f64).sqrt();
            distances[i * points.len() + j] = T::from(distance).unwrap();
        }
    }
    return distances;
}

#[cfg(test)]
pub fn build_geo_sample() -> Vec<u8> {
    build_geometric_distances(&vec![
        (12, 8),
        (20, 16),
        (24, 8),
        (32, 0),
        (40, 8),
        (40, 16),
        (16, 4),
    ])
}

#[cfg(test)]
pub fn check_permutation(items: &Vec<usize>, max: usize) {
    let unique_items: HashSet<_> = HashSet::from_iter(items);
    for i in 0..max + 1 {
        if !unique_items.contains(&i) {
            assert!(false, "candidate tour doesn't contain {:?}", i);
        }
    }
    if unique_items.len() != max + 1 {
        assert!(
            false,
            "candidate tour has the wrong number of items, expected={:?}, actual={:?}",
            max + 1,
            unique_items.len()
        );
    }
    if items.len() != max + 1 {
        assert!(
            false,
            "candidate tour has the wrong number of total items, expected={:?}, actual={:?}",
            max + 1,
            items.len()
        );
    }
}

#[cfg(test)]
pub fn build_4item_distances() -> Tsp<u8> {
    // goal is to have 1 close to 3 and 2 close to 4
    let mut vec = Vec::new();
    vec.append(&mut vec![0, 255, 1, 255]);
    vec.append(&mut vec![255, 0, 255, 1]);
    vec.append(&mut vec![1, 255, 0, 255]);
    vec.append(&mut vec![255, 1, 255, 0]);
    return Tsp::new(vec, 4);
}

#[cfg(test)]
pub fn build_5item_distances() -> Tsp<u8> {
    // goal is to have 1 close to 3 and 2 close to 4, 5 can be close to everything
    let mut vec = Vec::new();
    vec.append(&mut vec![0, 255, 1, 255, 1]);
    vec.append(&mut vec![255, 0, 255, 1, 1]);
    vec.append(&mut vec![1, 255, 0, 255, 1]);
    vec.append(&mut vec![255, 1, 255, 0, 1]);
    vec.append(&mut vec![1, 1, 1, 1, 0]);
    return Tsp::new(vec, 5);
}

#[cfg(test)]
pub fn read_tsp_file(path: PathBuf) -> Result<Option<Vec<(f32, f32)>>, std::io::Error> {
    let mut coordinates = Vec::new();
    for line in read_to_string(path)?.lines() {
        if line.starts_with("EDGE_WEIGHT_TYPE") {
            if !line.ends_with("EUC_2D") {
                return Ok(Option::None);
            }
        }
        let parts: Vec<&str> = line.split(" ").collect();
        if parts.len() >= 3 {
            let node_number = usize::from_str_radix(parts[0], 10);
            match node_number {
                Ok(_) => {
                    let x_coord = parts[1].parse();
                    let y_coord = parts[2].parse();
                    if x_coord.is_ok() && y_coord.is_ok() {
                        coordinates.push((x_coord.unwrap(), y_coord.unwrap()));
                    }
                }
                Err(_) => {
                    continue;
                }
            }
        }
    }
    Ok(Option::Some(coordinates))
}