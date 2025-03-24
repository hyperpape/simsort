# Simsort

Simsort is a tool for ordering files by similarity. When used as part
of creating a compressed tar archive, this can reduce the final size
of the archive. See [File Order Effects on
Compression](https://justinblank.com/notebooks/fileordereffectsoncompression.html).

## Usage

```simsort target-dir algorithm```

will generate an ordered list of files in the target directory. The algorithm 
can be one of:

- `only-extensions` groups files with the same extensions together
- `byte-distributions` groups files based on the distribution of bytes within them
- `tsp` attempts to order files by similarity

In principle `tsp` will take the most time, but give the best gains, while 
`only-extensions` will be the fastest.  

To create a tar file, you can use the command line:

```simsort target-dir algorithm | tar --no-recursion -cf archive.tar -T -```. 

## Implementation

Simsort started as a port of
[binsort](http://neoscientists.org/~tmueller/binsort/), an earlier
tool performing the same function, but has a handful of tweaks and has a few
optimizations that make it produce results faster and scale to larger archives.

The underlying idea is to produce a measure of binary similarity between files
(minhash), and then optimize a traveling salesman instance where two files 
have lower distance between them if they are more similar. 

Binary similarity for files is calculated using 
[Minhash](https://en.wikipedia.org/wiki/MinHash), and the tsp heuristic used is 
[2-opt](https://en.wikipedia.org/wiki/2-opt). Both choices are likely to
change.

### Large Archives

Since traveling salesman heuristics take superlinear time in the number
on archives with too many files. When the number of files in the
archive exceeds 10000, we first group files by extension. We then
build batches of <= 10000 files out of those groups, optimize each
batch separately, then concatenate them into a final ordering.

## Limitations

Though simsort generally is able to produce smaller archives than just running
tar with compression, there are cases (e.g. a checkout of the linux kernel),
where it produces worse results. I'm hoping to diagnose and fix such cases.