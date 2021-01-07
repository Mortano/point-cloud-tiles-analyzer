use crate::progress::ProgressUpdateCondition;
use anyhow::{anyhow, Result};
use core::fmt::Display;
use core::fmt::Formatter;
use las::{Read, Reader};
use rayon::prelude::*;
use signifix::metric;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::{convert::TryFrom, ops::Range};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};
use walkdir::WalkDir;

use crate::progress::ProgressTracker;

/// Generate histogram with logarithmic bucket size or linear bucket size?
pub enum HistogramConfig {
    Logarithmic(usize),
    Linear(usize),
}

fn log_histogram(counts: &[usize], num_buckets: usize) -> Vec<HistogramBucket> {
    let max_points = match counts.last() {
        None => return vec![],
        Some(&max_points) => max_points,
    };
    let log_max_points = (1.0 + max_points as f64).log2();

    // num_points_per_node is sorted, so we have to find the num_buckets-1 split positions where two buckets touch
    // We use logarithmic bucket sizes based on the maximum number of points
    let mut buckets = vec![];
    for bucket_index in 0..num_buckets {
        // Have to add 1 because log of 0 is -Inf
        let bucket_start = (2.0_f64
            .powf(log_max_points * (bucket_index as f64 / num_buckets as f64)))
        .round() as usize;
        let bucket_end = (2.0_f64
            .powf(log_max_points * ((bucket_index + 1) as f64 / num_buckets as f64)))
        .round() as usize;

        let first_match_index = counts.partition_point(|&count| count < bucket_start);
        let last_match_index = counts.partition_point(|&count| count < bucket_end);
        let count_in_bucket = last_match_index - first_match_index;

        buckets.push(HistogramBucket::new(
            count_in_bucket,
            bucket_start..bucket_end,
        ));
    }

    buckets
}

fn lin_histogram(counts: &[usize], num_buckets: usize) -> Vec<HistogramBucket> {
    let max_points = match counts.last() {
        None => return vec![],
        Some(&max_points) => max_points,
    } + 1;

    // num_points_per_node is sorted, so we have to find the num_buckets-1 split positions where two buckets touch
    let mut buckets = vec![];
    for bucket_index in 0..num_buckets {
        let bucket_start =
            (max_points as f64 * (bucket_index as f64 / num_buckets as f64)).round() as usize;
        let bucket_end =
            (max_points as f64 * ((bucket_index + 1) as f64 / num_buckets as f64)).round() as usize;

        let first_match_index = counts.partition_point(|&count| count < bucket_start);
        let last_match_index = counts.partition_point(|&count| count < bucket_end);
        let count_in_bucket = last_match_index - first_match_index;

        buckets.push(HistogramBucket::new(
            count_in_bucket,
            bucket_start..bucket_end,
        ));
    }

    buckets
}

/// Bucket within a Histogram containing the number of nodes whose point counts fall within `range`
#[derive(Debug)]
pub struct HistogramBucket {
    count: usize,
    range: Range<usize>,
}

impl HistogramBucket {
    /// Creates a new `HistogramBucket` with the given data
    /// ```
    /// # use crate::analyzer::*;
    /// // Create a new bucket containing 1024 entries that contain at least 50 and less than 100 points
    /// let bucket = HistogramBucket::new(1024, 50..100);
    /// ```
    pub fn new(count: usize, range: Range<usize>) -> Self {
        Self { count, range }
    }

    /// Returns the number of entries within the associated `HistogramBucket`
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns the range of the associated `HistogramBucket`
    pub fn range(&self) -> &Range<usize> {
        &self.range
    }
}

impl Display for HistogramBucket {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            fmt,
            "{} in [{};{})",
            self.count, self.range.start, self.range.end
        )
    }
}

/// Result of the `Analyzer`
pub enum AnalyzerResult {
    /// The number of nodes in the dataset
    NodeCount(usize),
    /// A histogram of the point counts for each node
    Histogram(Vec<HistogramBucket>),
}

impl Display for AnalyzerResult {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            AnalyzerResult::Histogram(histogram) => {
                writeln!(fmt, "Histogram:")?;
                for bucket in histogram.iter() {
                    writeln!(fmt, "{}", bucket)?;
                }
                Ok(())
            }
            AnalyzerResult::NodeCount(node_count) => {
                writeln!(fmt, "Number of nodes: {}", node_count)
            }
        }
    }
}

/// Trait for analyzing a point cloud
pub trait Analyzer {
    /// Runs the analyzer, returning the results of the analysis on success
    fn run(&self) -> Result<Vec<AnalyzerResult>>;
}

/// Analyzer for tiling formats where one node equals one file
pub struct MultiFileAnalyzer {
    files: Vec<PathBuf>,
    count_nodes: bool,
    histogram_config: Option<HistogramConfig>,
}

impl MultiFileAnalyzer {
    /// Creates a new `MultiFileAnalyzer` for the data in the given directory
    pub fn new<P: AsRef<Path>>(
        root_dir: P,
        count_nodes: bool,
        histogram_config: Option<HistogramConfig>,
    ) -> Result<Self> {
        if !root_dir.as_ref().exists() {
            return Err(anyhow!(
                "root directory {} does not exist!",
                root_dir.as_ref().display()
            ));
        }

        let files = WalkDir::new(root_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| Self::is_supported_format(entry.path()))
            .map(|entry| entry.into_path())
            .collect::<Vec<_>>();

        Ok(MultiFileAnalyzer {
            files,
            count_nodes,
            histogram_config,
        })
    }

    fn is_supported_format<P: AsRef<Path>>(path: P) -> bool {
        match path.as_ref().extension() {
            Some(extension) => extension == "las" || extension == "laz",
            None => false,
        }
    }

    fn calculate_histogram(&self) -> Result<AnalyzerResult> {
        let progress_chunk_size = 1024;
        let mut progress_tracker = Arc::new(Mutex::new(ProgressTracker::new(
            (self.files.len() - 1) as f64,
            ProgressUpdateCondition::OnProgressChanged(1000.0),
        )));

        let mut num_points_per_node = self
            .files
            .par_iter()
            .map(move |file| -> Result<usize> {
                let mut reader = Reader::from_path(file)?;
                let header = reader.header();
                let num_points = header.number_of_points() as usize;
                let mut progress = progress_tracker.lock().unwrap();
                progress.inc_progress(1.0);
                Ok(num_points)
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Generate histogram from num_points_per_node
        num_points_per_node.sort();

        let histogram = match self.histogram_config.as_ref().unwrap() {
            HistogramConfig::Linear(buckets) => {
                lin_histogram(num_points_per_node.as_slice(), *buckets)
            }
            HistogramConfig::Logarithmic(buckets) => {
                log_histogram(num_points_per_node.as_slice(), *buckets)
            }
        };

        Ok(AnalyzerResult::Histogram(histogram))
    }
}

impl Analyzer for MultiFileAnalyzer {
    fn run(&self) -> Result<Vec<AnalyzerResult>> {
        if self.files.is_empty() {
            return Err(anyhow!(
                "Found zero files to analyze! Make sure the target directory is not empty!"
            ));
        }

        eprintln!("Analyzing {} files in Entwine format", self.files.len());

        let mut results = vec![];
        if self.count_nodes {
            eprintln!("Counting nodes");
            results.push(AnalyzerResult::NodeCount(self.files.len()));
        }

        if self.histogram_config.is_some() {
            eprintln!("Calculating histogram");
            let histogram = self.calculate_histogram()?;
            results.push(histogram);
        }

        return Ok(results);
    }
}

/// Analyzer for the file format of PotreeConverter v2
pub struct PotreeV2FormatAnalyzer {
    hierarchy_file: PathBuf,
    count_nodes: bool,
    histogram_config: Option<HistogramConfig>,
}

impl PotreeV2FormatAnalyzer {
    pub fn new<P: AsRef<Path>>(
        root_dir: P,
        count_nodes: bool,
        histogram_config: Option<HistogramConfig>,
    ) -> Result<Self> {
        if !root_dir.as_ref().exists() {
            return Err(anyhow!(
                "root directory {} does not exist!",
                root_dir.as_ref().display()
            ));
        }

        let hierarchy_file = root_dir.as_ref().to_owned().join("hierarchy.bin");
        if !hierarchy_file.exists() {
            return Err(anyhow!("hierarchy.bin file does not exist!",));
        }

        Ok(Self {
            hierarchy_file,
            count_nodes,
            histogram_config,
        })
    }
}

impl Analyzer for PotreeV2FormatAnalyzer {
    fn run(&self) -> Result<Vec<AnalyzerResult>> {
        eprintln!("Analyzing dataset in PotreeConverter v2 format");

        let mut results = vec![];

        if !self.count_nodes && self.histogram_config.is_none() {
            return Ok(results);
        }

        let mut reader = BufReader::new(File::open(&self.hierarchy_file)?);
        let mut bytes = vec![];
        std::io::Read::read_to_end(&mut reader, &mut bytes)?;

        let size_of_node = 22;
        if bytes.len() % size_of_node != 0 {
            return Err(anyhow!(
                "File size of hierarchy.bin must be a multiple of {}!",
                size_of_node
            ));
        }

        let num_node_entries = bytes.len() / size_of_node;
        let valid_node_indices = (0..num_node_entries)
            .filter(|idx| bytes[idx * size_of_node] != 2 || bytes[idx * size_of_node + 1] == 0)
            .collect::<Vec<_>>();

        if self.count_nodes {
            results.push(AnalyzerResult::NodeCount(valid_node_indices.len()));
        }

        if self.histogram_config.is_some() {
            let mut points_per_node = vec![];
            for node_idx in valid_node_indices.iter() {
                let offset_to_size_in_bytes = (node_idx * size_of_node) + 2;
                let point_count_of_node = u32::from_le_bytes([
                    bytes[offset_to_size_in_bytes],
                    bytes[offset_to_size_in_bytes + 1],
                    bytes[offset_to_size_in_bytes + 2],
                    bytes[offset_to_size_in_bytes + 3],
                ]);
                points_per_node.push(point_count_of_node as usize);
            }
            points_per_node.sort();

            let histogram = match self.histogram_config.as_ref().unwrap() {
                HistogramConfig::Linear(buckets) => {
                    lin_histogram(points_per_node.as_slice(), *buckets)
                }
                HistogramConfig::Logarithmic(buckets) => {
                    log_histogram(points_per_node.as_slice(), *buckets)
                }
            };
            results.push(AnalyzerResult::Histogram(histogram));
        }

        Ok(results)
    }
}
