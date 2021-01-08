// mean and std_deviation taken from https://rust-lang-nursery.github.io/rust-cookbook/science/mathematics/statistics.html

pub fn mean(data: &[usize]) -> Option<f64> {
    let sum = data.iter().sum::<usize>() as f64;
    let count = data.len();

    match count {
        positive if positive > 0 => Some(sum / count as f64),
        _ => None,
    }
}

/// Computes the mean and standard deviation of the given values
pub fn mean_and_std_deviation(data: &[usize]) -> Option<(f64, f64)> {
    match (mean(data), data.len()) {
        (Some(data_mean), count) if count > 0 => {
            let variance = data
                .iter()
                .map(|value| {
                    let diff = data_mean - (*value as f64);

                    diff * diff
                })
                .sum::<f64>()
                / count as f64;

            Some((data_mean, variance.sqrt()))
        }
        _ => None,
    }
}
