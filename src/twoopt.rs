use rand::thread_rng;
use rand::Rng;

use crate::tour::Tour;
use crate::tsp::Tsp;
use crate::utils;
use num_traits::int::PrimInt;

const MOD_COUNT: usize = 20;

pub const MINIMUM_ITEMS: usize = 3;

pub fn optimize_twoopt<T: PrimInt>(tsp: &Tsp<T>) -> Result<Vec<usize>, String> {
    let mut rng = thread_rng();
    assert!(
        tsp.count >= MINIMUM_ITEMS,
        "Cannot optimize a tsp with less than 3 items"
    );
    let starting_index = rng.gen_range(1..tsp.count);
    return optimize_twoopt_from_index(&tsp, starting_index);
}

fn optimize_twoopt_from_index<T: PrimInt>(
    tsp: &Tsp<T>,
    starting_index: usize,
) -> Result<Vec<usize>, String> {
    let tour = Tour::new(tsp.generate_nearest_neighbor_tour(starting_index));
    optimize_twoopt_from_tour(tsp, tour)
}

pub fn optimize_twoopt_from_tour<T: PrimInt>(
    tsp: &Tsp<T>,
    mut tour: Tour,
) -> Result<Vec<usize>, String> {
    let mut improvement_count = 0;
    let mut distance = tsp.calculate_distance_tour(&tour);
    let original_distance = distance;

    loop {
        if improvement_count % MOD_COUNT == 0 {
            utils::perf_trace("Improve TwoOpt", "Optimize", "B", utils::get_micros());
        }
        let mut improved = false;
        for i in 0..tsp.count {
            match improve(&tsp, &mut tour, i)? {
                true => {
                    let new_distance = tsp.calculate_distance_tour(&tour);
                    // TODO: remove distance check, once tests are passing
                    // println!("Update. Tour={:?}, new_distance={}, distance={}", &tour, new_distance, distance);
                    assert!(new_distance <= distance);
                    // TODO: this is a hack, we should be able to avoid this by construction
                    if new_distance < distance {
                        distance = new_distance;
                        improved = true;
                    }
                    // log::info!("Distance={}, NewDistance={}")
                }
                false => {}
            }
        }
        improvement_count += 1;
        if improvement_count % MOD_COUNT == 0 {
            utils::perf_trace("Improve TwoOpt", "Optimize", "E", utils::get_micros());
        }

        if !improved {
            if improvement_count % MOD_COUNT != 0 {
                utils::perf_trace("Improve TwoOpt", "Optimize", "E", utils::get_micros());
            }
            log::debug!("StartingDistance={}, EndingDistance={}", original_distance, tsp.calculate_distance_tour(&tour));
            return Ok(tour.to_indices());
        }
    }
}

fn improve<T: PrimInt>(tsp: &Tsp<T>, tour: &mut Tour, base: usize) -> Result<bool, String> {
    let next_base = tour.next(base);
    let edge_distance = tsp.retrieve_distance(base, next_base);
    // TODO: measure time/efficiency tradeoff of doing neighors vs. all vertexes
    for candidate in &tsp.neighbors[base] {
        if tour.are_neighbors(base, *candidate) {
            continue;
        }
        let new_distance = tsp.retrieve_distance(base, *candidate);
        for candidate_neighbor in vec![tour.next(*candidate)] {
            // include tour.prev?
            if candidate_neighbor == base || candidate_neighbor == next_base {
                continue;
            }
            let neighbor_distance = tsp.retrieve_distance(*candidate, candidate_neighbor);
            let rejoin_distance = tsp.retrieve_distance(candidate_neighbor, next_base);
            if edge_distance + neighbor_distance > rejoin_distance + new_distance {
                // println!("--------------------------------");
                // println!("        Tour={:?}", &tour);
                // println!("        Current_Distance={}", &tsp.calculate_distance_tour(&tour));
                // println!("        Base={}, Candidate={}, edge_distance={}, new_distance={}, neighbor_distance={}, rejoin_distance={}",
                //         base, candidate, edge_distance, new_distance, neighbor_distance, rejoin_distance);
                // check which one follows...
                if tour.simple_between(*candidate, base, candidate_neighbor) {
                    tour.flip(base, candidate_neighbor);
                } else {
                    tour.flip(next_base, *candidate);
                }
                assert!(tour.are_neighbors(base, *candidate));
                assert!(tour.are_neighbors(next_base, candidate_neighbor));
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use crate::twoopt::*;
    use crate::testutils::*;

    #[test]
    fn test_from_tsp_lib() {
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let file_path = project_root.join("testdata/berlin52.tsp");
        let coordinates = read_tsp_file(file_path)
            .unwrap()
            .unwrap();
        let distances: Vec<u32> = build_geometric_distances_alt(coordinates);
        let count = 52;
        assert!(count * count == distances.len());
        let tsp = Tsp::new(distances, count);
        let result = optimize_twoopt_from_index(&tsp, 11).unwrap();
        let calculated_distance = tsp.calculate_distance(&result);
        assert!(calculated_distance < 8000);
    }
}
