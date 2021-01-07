#![feature(partition_point)]

use crate::analyzer::Analyzer;
use crate::analyzer::HistogramConfig;
use crate::analyzer::MultiFileAnalyzer;
use analyzer::PotreeV2FormatAnalyzer;
use anyhow::{anyhow, Result};
use clap::{value_t, App, Arg};
use std::str::FromStr;

use std::path::{Path, PathBuf};

mod analyzer;
mod progress;

struct Config {
    input_dir: PathBuf,
    count_nodes: bool,
    histogram_config: Option<HistogramConfig>,
}

fn get_config() -> Result<Config> {
    let matches = App::new("Point cloud tiles analyzer")
        .version("1.0")
        .author("Pascal Bormann")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("DIR")
                .help("The path to the directory of the tiled point cloud. Supported formats are PotreeConverter v1.7, PotreeConverter v2, Entwine and Schwarzwald")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("count_nodes")
                .short("c")
                .long("count-nodes")
                .help("Count the number of nodes in the tiled point cloud")
        )
        .arg(
            Arg::with_name("histogram_lin")
            .long("histogram-lin")
            .help("Calculate a histogram of the number of points in each node with the specified number of buckets. Bucket size will be linear between 1 and the maximum number points in a node")
            .takes_value(true)
        )
        .arg(Arg::with_name("histogram_log")
        .long("histogram-log")
        .help("Calculate a histogram of the number of points in each node with the specified number of buckets. Bucket size will be logarithmic between 1 and the maximum number points in a node")
        .takes_value(true))
        .get_matches();

    let file = matches
        .value_of("input")
        .expect("Argument --input was missing!");
    let path = PathBuf::from_str(file)?;

    let count_nodes = matches.is_present("count_nodes");
    let calculate_linear_histogram = matches.is_present("histogram_lin");
    let calculate_logarithmic_histogram = matches.is_present("histogram_log");
    if calculate_linear_histogram && calculate_logarithmic_histogram {
        panic!("Arguments histogram-lin and histogram-log are mutually exclusive!");
    }
    let histogram_config = if calculate_linear_histogram {
        Some(HistogramConfig::Linear(value_t!(
            matches,
            "histogram_lin",
            usize
        )?))
    } else if calculate_logarithmic_histogram {
        Some(HistogramConfig::Logarithmic(value_t!(
            matches,
            "histogram_log",
            usize
        )?))
    } else {
        None
    };

    Ok(Config {
        input_dir: path,
        count_nodes,
        histogram_config,
    })
}

fn is_entwine_dataset(root_dir: &Path) -> bool {
    let ept_data_dir = root_dir.to_owned().join("ept-data");
    ept_data_dir.exists()
}

fn is_potree_legacy_dataset(root_dir: &Path) -> bool {
    let cloud_js_path = root_dir.to_owned().join("cloud.js");
    cloud_js_path.exists()
}

fn is_potree_v2_dataset(root_dir: &Path) -> bool {
    let hierarchy_bin_path = root_dir.to_owned().join("hierarchy.bin");
    hierarchy_bin_path.exists()
}

fn make_analyzer(config: Config) -> Result<Box<dyn Analyzer>> {
    if is_entwine_dataset(&config.input_dir) || is_potree_legacy_dataset(&config.input_dir) {
        let ept_data_dir = config.input_dir.to_owned().join("ept-data");
        let analyzer =
            MultiFileAnalyzer::new(ept_data_dir, config.count_nodes, config.histogram_config)?;
        Ok(Box::new(analyzer))
    } else if is_potree_v2_dataset(&config.input_dir) {
        let analyzer = PotreeV2FormatAnalyzer::new(
            config.input_dir,
            config.count_nodes,
            config.histogram_config,
        )?;
        Ok(Box::new(analyzer))
    } else {
        Err(anyhow!("Tiling format not recognized!"))
    }
}

fn main() -> Result<()> {
    let config = get_config()?;
    let analyzer = make_analyzer(config)?;
    let results = analyzer.run()?;
    results.iter().for_each(|result| print!("{}", result));

    Ok(())
}
