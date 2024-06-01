use rand::Rng;

use crate::tsp::Tsp;

const BINSORT_DEFAULT_QUALITY: u8 = 15;

pub fn optimize_binsort(tsp: &Tsp<u8>) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..tsp.count).collect();
    let file_count = tsp.count;
    let initial_distance = tsp.calculate_distance(&indices);
    log::info!(
        "Created distances for {} files. Initial distance is {}",
        file_count,
        initial_distance
    );
    let iteration_count =
        ((initial_distance as f64).powf(1.1) * BINSORT_DEFAULT_QUALITY as f64) as i64;

    log::info!("iteration_count is {}", iteration_count);
    // no idea what dunk means, it's used for computing the acceptance threshold
    let dunk = (initial_distance as f64) * 1.1 / iteration_count as f64;
    let mut rng = rand::thread_rng();
    let mut distance = initial_distance;
    for i in 0..iteration_count as u32 {
        let mut thresh = (initial_distance as f64) / (i as f64 - dunk);
        if thresh < 0.0 {
            thresh = 0.0;
        }
        log::debug!(
            "thresh is {}, on iteration {} of {}",
            thresh,
            i,
            iteration_count
        );
        if i & 65535 == 0 {
            log::info!("on iteration {} of {}", i, iteration_count);
        }
        loop {
            // generate some indexes...but how/why?
            let mut i0: usize = rng.gen_range(0..file_count);
            let mut i1: usize = rng.gen_range(0..file_count);
            let mut n: usize = 0;
            log::debug!("Index generating values are {}, {}, {}", i0, i1, n);
            if i0 >= i1 + 2 {
                n = file_count - i0 + i1 + 1;
                if n > i0 - i1 - 1 {
                    // reverse order, shrink interval and set n to gap
                    let t = i1 + 1;
                    i1 = i0 - 1;
                    i0 = t;
                    n = i1 - i0 + 1;
                }
            } else if i1 > i0 && i1 - i0 <= file_count - 2 {
                if file_count - i1 + i0 - 1 < i1 - i0 + 1 {
                    // case where gap between i1 and i0 is large
                    let t = if i0 == 0 { file_count - 1 } else { i0 - 1 };
                    i0 = (i1 + 1) % file_count;
                    i1 = t;
                    if i0 > i1 {
                        n = file_count - i0 + i1 + 1;
                    } else {
                        n = i1 - i0 + 1;
                    }
                } else {
                    // set n to gap between i1 and i0
                    n = i1 - i0 + 1;
                }
            } else {
                // we regenerate the indices when either:
                // 1) first index is right after the second
                // 2) first index is 0 and/or second index is file_count - 1
                continue;
            }
            let i11 = (i1 + 1) % file_count;
            let i00 = if i0 == 0 { file_count - 1 } else { i0 - 1 };
            // locking code goes here

            let delta = getdelta(&tsp, &indices, i0, i1, i00, i11);
            if delta < thresh as i64 {
                distance = (distance as i64 + delta) as u64;
                let t = indices[i0];
                indices[i0] = indices[i1];
                indices[i1] = t;

                if n > 3 {
                    // update distance
                    // do something with rangelocks;
                    // addTail

                    i0 = (i0 + 1) % file_count;
                    if i1 <= 0 {
                        i1 += file_count - 1;
                    } else {
                        i1 -= 1;
                    }
                    // TODO: I think this was supposed to mutate the i in the enclosing scope or something?
                    for i in 1..n / 2 {
                        let t = indices[i0];
                        indices[i0] = indices[i1];
                        indices[i1] = t;
                        i0 = (i0 + 1) % file_count;
                        if i1 <= 0 {
                            i1 += file_count - 1;
                        } else {
                            i1 -= 1;
                        }
                    }
                }
            }
            break;
        }
    }
    return indices;
}

fn getdelta(
    tsp: &Tsp<u8>,
    indices: &Vec<usize>,
    i0: usize,
    i1: usize,
    i00: usize,
    i11: usize,
) -> i64 {
    let mut delta: i64 = 0;
    let a = indices[i0];
    let b = indices[i1];
    let c = indices[i00];
    let d = indices[i11];
    // TODO: I don't understand how a could be < 0
    if a > 0 {
        // TODO: binsort uses >= here
        if c > 0 {
            delta -= tsp.retrieve_distance(a, c) as i64;
        }
        if d > 0 {
            delta += tsp.retrieve_distance(a, d) as i64;
        }
    }
    if b > 0 {
        if d > 0 {
            delta -= tsp.retrieve_distance(b, d) as i64;
        }
        if c > 0 {
            delta += tsp.retrieve_distance(b, c) as i64;
        }
    }

    return delta;
}

#[cfg(test)]
mod tests {
    use crate::binsort::*;
    use crate::testutils::*;

    #[test]
    fn optimize_binsort_can_optimize() {
        let tsp = build_4item_distances();
        assert_eq!(255 * 3, tsp.calculate_distance(&vec!(0, 1, 2, 3)));

        let optimized = optimize_binsort(&tsp);
        let optimized_distance = tsp.calculate_distance(&optimized);
        check_permutation(&optimized, 3);
        // TODO: decide if this is reasonable--should we really be seeing 510 frequently?
        assert!(257 == optimized_distance || 511 == optimized_distance);
    }
}
