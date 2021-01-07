# Point cloud tiles analyzer

Analyze tiled point cloud datasets. Supports the following point cloud tiling systems:

- [Schwarzwald](https://github.com/igd-geo/schwarzwald)
- [PotreeConverter](https://github.com/potree/PotreeConverter) (v1.7 and v2)
- [Entwine](https://entwine.io/)

## Build

**Requires Rust nightly to build!** In the root directory, run the following commands to build:
```
rustup install nightly
cargo +nightly build
```

## Usage

Run the tool with the argument `--input TARGET_DIR`, where `TARGET_DIR` is the root directory that contains your tiled point cloud. Currently, two analysis modes are supported:
- Counting the total number of nodes in the tiled point cloud (enabled through `--count-nodes`)
- Generating a histogram which displays the number of points that each node contains. Enabled through `--histogram-lin NUM_BUCKETS` for a histogram with linear bucket size, or `--histogram-log NUM_BUCKETS` for logarithmic bucket size