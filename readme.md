# Simsort

Simsort is a tool for ordering files by similarity. When used as part
of creating a compressed tar archive, this can reduce the final size
of the archive. See [File Order Effects on
Compression](https://justinblank.com/notebooks/fileordereffectsoncompression.html).

The effect is most significant when the archive contains perfect
duplicates, or when using gzip instead of zstd, but the effect
persists even when using zstd with archives that contain no
duplicates.

## Usage

## Implementation

Simsort started as a port of
[binsort](http://neoscientists.org/~tmueller/binsort/), an earlier
tool performing the same function, but implements a different
algorithm and has a few optimizations that make it produce results
faster and scale to larger archives.

To order files by binary similarity, simsort implements the
lin-kernighan heuristic for the traveling salesman problem (though I
have not yet finished implementing one step).  

### Large Archives

Since the LK heuristic has an O(n^2) complexity, it becomes quite slow
on archives with too many files. When the number of files in the
archive exceeds 10000, we first group files by extension. We then
build batches of <= 10000 files out of those groups, optimize each
batch separately, then concatenate them into a final ordering.