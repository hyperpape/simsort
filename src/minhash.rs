use std::collections::BinaryHeap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, Error, Read};
use std::path::PathBuf;

use crc32fast;

const SHINGLE_SIZE: u8 = 8;
const FEATURE_COUNT: u8 = 128;

/**
 * This module is based on the code in binsort, but changes the name from simhash to minhash,
 * because I believe that it was misnamed.
 */

#[derive(Debug)]
pub struct Minhash {
    features: Vec<u32>,
    feature_count: u8,
    shingle_size: u8,
    // These values are not in the binsort implementation, they're my addition
    pub byte_distribution: ByteDistribution,
}

impl Minhash {
    fn new() -> Minhash {
        return Minhash {
            features: Vec::new(),
            feature_count: FEATURE_COUNT,
            shingle_size: SHINGLE_SIZE,
            byte_distribution: ByteDistribution::Uniform,
        };
    }

    /* BINSORT COMMENT walk backward until one set runs out, counting the
    number of elements in the union of the sets.  the
    backward walk is necessary because the common subsets
    are at the end of the file by construction.  bleah.
    should probably reformat so that it's the other way
    around, which would mean that one could shorten a
    shingleprint by truncation. */

    pub fn score(&self, h2: &Minhash) -> f64 {
        let mut i1 = self.features.len() - 1;
        let mut i2 = h2.features.len() - 1;
        let mut matchcount = 0;

        loop {
            if self.features[i1] < h2.features[i2] {
                if i2 == 0 {
                    break;
                }
                i2 -= 1;
                continue;
            }
            if self.features[i1] > h2.features[i2] {
                if i1 == 0 {
                    break;
                }
                i1 -= 1;
                continue;
            }
            matchcount += 1;
            if i1 == 0 || i2 == 0 {
                break;
            }
            i1 -= 1;
            i2 -= 1;
        }
        let count = std::cmp::min(self.features.len(), h2.features.len());
        let unionsize = 2 * count - matchcount;
        return (matchcount as f64) / (unionsize as f64);
    }
}

pub fn minhash_stream(target: &MinhashTarget) -> Result<Minhash, Error> {

    let mut minhash = Minhash::new();
    let mut heap: BinaryHeap<u32> = BinaryHeap::new();
    let mut buf = Vec::new();
    let mut filled_buf: bool = false;
    let path = target.get_path();

    let mut filename_byte_distribution = ByteCount::new();
    for &b in path.as_os_str().as_encoded_bytes() {
        filename_byte_distribution.record_byte(b);
        if !filled_buf {
            buf.push(b);
            if buf.len() == minhash.shingle_size as usize {
                filled_buf = true;
            } else {
                continue;
            }
        } else {
            // TODO: we can save some time by using a slice into a larger buffer
            push_back(&mut buf, b);
        }
        let hash = crc32fast::hash(&buf);
        shingle_update(&mut minhash, &mut heap, hash);
    }
    // We purposefully allow this distribution to be overridden if we're hashing a file
    // the idea is that not doing that would make many files show up as non-random.
    // It might actually be correct to do that--a small binary file would get treated as
    // non-uniform, while a larger one would be uniform, but that's an improvement for later.
    minhash.byte_distribution = filename_byte_distribution.to_distribution();

    match target {
        MinhashTarget::Directory(_) => {}
        MinhashTarget::File(_) => {
            let byte_count = shingle_file(&mut minhash, &mut heap, buf, filled_buf, path)?;
            minhash.byte_distribution = byte_count.to_distribution();
        }
    }

    minhash.features = heap.into_sorted_vec();
    Ok(minhash)
}

fn shingle_file(minhash: &mut Minhash, heap: &mut BinaryHeap<u32>, mut buf: Vec<u8>, mut filled_buf: bool, path: &PathBuf) -> Result<ByteCount, Error> {
    let f = File::open(path)?;
    let mut reader = BufReader::new(f);
    let mut buffer = [0; 1024];
    let mut byte_count = ByteCount::new();
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        for &b in &buffer[..n] {
            byte_count.record_byte(b);
            if !filled_buf {
                buf.push(b);
                if buf.len() == minhash.shingle_size as usize {
                    filled_buf = true;
                } else {
                    continue;
                }
            } else {
                // TODO: we can save some time by using a slice into a larger buffer
                push_back(&mut buf, b);
            }

            let hash = crc32fast::hash(&buf);
            shingle_update(minhash, heap, hash);
        }
    }
    Ok(byte_count)
}

// Difference from the binsort implementation--it used a circular buffer, meaning for the string 123456789ABCDEF, it would hash
// 12345678 then 92345678 then 9A345678...I don't see a purpose in doing that
fn push_back(buf: &mut Vec<u8>, byte: u8) {
    for i in 0..buf.len() - 1 {
        buf[i] = buf[i + 1];
    }
    let index = buf.len() - 1;
    buf[index] = byte;
}

fn shingle_update(minhash: &mut Minhash, heap: &mut BinaryHeap<u32>, hash: u32) {
    let mut hashes = HashSet::new();

    match heap.peek() {
        Some(&h) => {
            if heap.len() == minhash.feature_count as usize {
                if h > hash {
                    hashes.insert(hash);
                    hashes.remove(&h);
                    heap.pop();
                    heap.push(hash);
                }
            } else if !hashes.contains(&hash) {
                hashes.insert(hash);
                heap.push(hash);
            }
        }
        None => {
            heap.push(hash);
            hashes.insert(hash);
        }
    }
}

struct ByteCount {
    count: u64,
    bytes: Vec<u32>,
}

impl ByteCount {
    fn new() -> ByteCount {
        ByteCount {
            count: 0,
            bytes: vec![0; 256],
        }
    }

    fn record_byte(&mut self, byte: u8) {
        self.bytes[byte as usize] += 1;
        self.count += 1;
    }

    fn ascii(&self) -> bool {
        for i in 128..256 {
            if self.bytes[i] > 0 {
                return false;
            }
        }
        true
    }

    /**
     * Determine how many bytes are present in the file, in order to pre-sort files (e.g. two binary files that use all 256 byte values are more similar to each other than to an ascii file).
     * A more robust solution would probably be to calculate some sort of statistical estimate of whether the bytes in the file are random looking.
     */
    fn bytes_present(&self) -> u16 {
        let mut seen = 0;
        for i in 0..256 {
            if self.bytes[i] > 0 {
                seen += 1;
            }
        }
        seen
    }

    fn to_distribution(self) -> ByteDistribution {
        let bytes_present = self.bytes_present();
        if self.ascii() {
            ByteDistribution::Ascii((bytes_present, Box::new(self.bytes)))
        } else if self.is_uniform() {
            ByteDistribution::Uniform
        } else {
            ByteDistribution::NonAscii((bytes_present, Box::new(self.bytes)))
        }
    }

    fn is_uniform(&self) -> bool {
        let chi_squared = self.variance() / 256.0;
        chi_squared < 254.0 // 50% with 255 degrees of freedom
    }

    fn variance(&self) -> f32 {
        let mut variance: f32 = 0.0;
        let expected = ((self.count as f64) / 256.0) as f32;
        for i in 0..255 {
            let count = self.bytes[i];
            let difference = (count as f32 - expected).abs();
            variance += difference * difference;
        }
        variance / expected
    }
}

#[derive(Debug)]
pub enum ByteDistribution {
    // Represents a set of bytes that match a uniform distribution for frequency. Does not test for randomness. [1, 2, 3, 4, 5, 6...] would count as uniform.
    Uniform,
    NonAscii((u16, Box<Vec<u32>>)),
    Ascii((u16, Box<Vec<u32>>)),
}

#[derive(Clone, Debug)]
pub enum MinhashTarget {
    Directory(PathBuf),
    File(PathBuf),
}

impl MinhashTarget {
    pub fn get_path(&self) -> &PathBuf {
        match self {
            MinhashTarget::Directory(p) => p,
            MinhashTarget::File(p) => p,
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::minhash::*;
    use proptest::prelude::*;
    use rand::rngs::StdRng;
    use rand::Rng;
    use rand::SeedableRng;
    use std::fs::File;
    use std::io::Write;
    use tempfile::{tempdir, NamedTempFile};

    fn write_to_temp_file(bytes: &Vec<u8>) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(bytes).unwrap();
        file
    }

    fn shingle_bytes(bytes: &Vec<u8>) -> Minhash {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(bytes).unwrap();
        let temp_path = file.into_temp_path();
        let mut minhash = Minhash::new();
        let mut heap = BinaryHeap::new();
        shingle_file(&mut minhash, &mut heap, Vec::new(), false, &temp_path.to_path_buf()).unwrap();
        minhash.features = heap.into_sorted_vec();
        minhash
    }

    #[test]
    fn lengthening_string_adds_shingle_features() {
        let mut buf =
            "This is a string that has enough characters that we should be able to shingle it"
                .as_bytes()
                .to_vec();
        let minhash = shingle_bytes(&buf);

        buf.extend_from_slice("Now we add more text".as_bytes());
        let minhash2 = shingle_bytes(&buf);

        assert!(minhash2.features.len() > minhash.features.len());
    }

    #[test]
    fn shingle_buffer_is_size_limited() {
        let mut rng = rand::thread_rng();
        let buf = (0..65536).map(|_| rng.gen()).collect();
        let minhash = shingle_bytes(&buf);
        assert_eq!(FEATURE_COUNT as usize, minhash.features.len());
    }

    #[test]
    fn shingling_twice_gives_same_length() {
        let buf =
            "This is a string that has enough characters that we should be able to shingle it"
                .as_bytes()
                .to_vec();
        let minhash1 = shingle_bytes(&buf);
        let minhash2 = shingle_bytes(&buf);
        assert_eq!(minhash1.features.len(), minhash2.features.len());
    }

    #[test]
    fn shingling_twice_gives_same_features() {
        let buf =
            "This is a string that has enough characters that we should be able to shingle it"
                .as_bytes()
                .to_vec();
        let file = write_to_temp_file(&buf);
        let temp_path = file.into_temp_path();
        let minhash1 = minhash_stream(&MinhashTarget::File(temp_path.to_path_buf())).unwrap();
        let minhash2 = minhash_stream(&MinhashTarget::File(temp_path.to_path_buf())).unwrap();
        assert_eq!(minhash1.features, minhash2.features);
    }

    #[test]
    fn shingle_compares_equal_to_self() {
        let buf =
            "This is a string that has enough characters that we should be able to shingle it"
                .as_bytes()
                .to_vec();
        let minhash = shingle_bytes(&buf);
        assert_eq!(1 as f64, minhash.score(&minhash));
    }

    #[test]
    fn distinct_strings_compare_unequal() {
        let buf1 =
            "This is a string that has enough characters that we should be able to shingle it"
                .as_bytes()
                .to_vec();
        let buf2 = "This is another string that has a different ending"
            .as_bytes()
            .to_vec();
        let minhash1 = shingle_bytes(&buf1);
        let minhash2 = shingle_bytes(&buf2);
        let cmp = minhash1.score(&minhash2);
        let reversed_cmp = minhash2.score(&minhash1);
        assert_eq!(cmp, reversed_cmp);
        assert!(cmp < 1.0);
    }

    #[test]
    fn disjoint_strings_compare_as_zero() {
        let buf1 = "a".repeat(128).as_bytes().to_vec();
        let buf2 = "b".repeat(128).as_bytes().to_vec();
        let minhash1 = shingle_bytes(&buf1);
        let minhash2 = shingle_bytes(&buf2);
        assert!(0.1 > minhash1.score(&minhash2));
    }

    #[test]
    fn overlapping_for_less_than_8bytes_does_not_count() {
        let buf1 = "ha".repeat(64).as_bytes().to_vec();
        let buf2 = "a".repeat(128).as_bytes().to_vec();
        let minhash1 = shingle_bytes(&buf1);
        let minhash2 = shingle_bytes(&buf2);
        assert_eq!(0 as f64, minhash1.score(&minhash2));
    }

    // TODO proptest
    // proptest! {
    #[test]
    fn random_distribution_appears_uniform() {
        let seed = [
            1, 0, 0, 0, 23, 0, 0, 0, 200, 1, 0, 0, 210, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0,
        ];

        let mut rng = StdRng::from_seed(seed);

        let mut byte_count = ByteCount::new();

        for _ in 0..255 {
            byte_count.record_byte((rng.next_u32() & 255) as u8);
        }

        match byte_count.to_distribution() {
            ByteDistribution::Uniform => {}
            _ => {
                panic!("Non uniform value")
            }
        }
    }
    // }

    proptest! {
        #[test]
        fn constant_distribution_appears_non_uniform(count in 0..255) {
            let mut byte_count = ByteCount::new();

            for _ in 0..255 {
                byte_count.record_byte(count as u8);
            }

            match byte_count.to_distribution() {
                ByteDistribution::Uniform => { panic!("Value is not uniform!")},
                _ => { }
            }
        }
    }

    #[test]
    fn minhash_stream_reads_file() {
        let temp_dir = tempdir().unwrap();
        let file_name: String = "xyz".to_string();
        let file_path = temp_dir.path().join(file_name);
        let mut file = File::create(file_path.clone()).unwrap();
        for _ in 0..1024 {
            write!(file, "a").unwrap();
        }
        let result: Result<Minhash, Error> =
            minhash_stream(&MinhashTarget::File(file_path.clone().to_path_buf()));
        match result {
            Ok(m) => match m.byte_distribution {
                ByteDistribution::Ascii(_) => {}
                _ => {
                    assert!(false);
                }
            },
            Err(_) => {
                assert!(false);
            }
        }
    }

    #[test]
    fn minhash_stream_generates_8_features() {
        let temp_dir = tempdir().unwrap();
        let file_name: String = "file1".to_string();
        let file_path = temp_dir.path().join(file_name);
        let mut file = File::create(file_path.clone()).unwrap();
        // Because multiple entries are recorded in the heap for the same value, our maximum is 128/8 here.
        for _ in 0..16 {
            write!(file, "abcdefgh").unwrap();
        }

        let minhash_1 =
            minhash_stream(&MinhashTarget::File(file_path.clone().to_path_buf())).unwrap();

        let feature_set: HashSet<u32> = HashSet::from_iter(minhash_1.features.into_iter());
        assert!(feature_set.len() >= 8);
    }

    #[test]
    fn minhash_sees_through_single_byte_offset() {
        let temp_dir = tempdir().unwrap();
        let file_name: String = "file1".to_string();
        let file_path = temp_dir.path().join(file_name);
        let mut file = File::create(file_path.clone()).unwrap();
        for _ in 0..1024 {
            write!(file, "abcdefgh").unwrap();
        }

        let file_name2 = "file2".to_string();
        let file_path2 = temp_dir.path().join(file_name2);
        let mut file2 = File::create(file_path2.clone()).unwrap();
        write!(file2, "1").unwrap();
        for _ in 0..1024 {
            write!(file2, "abcdefgh").unwrap();
        }
        let minhash_1 =
            minhash_stream(&MinhashTarget::File(file_path.clone().to_path_buf())).unwrap();
        let minhash_2 =
            minhash_stream(&MinhashTarget::File(file_path2.clone().to_path_buf())).unwrap();
        let score = minhash_1.score(&minhash_2);

        assert!(score > 0.95);
    }
}
