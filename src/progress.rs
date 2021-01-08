use std::{collections::VecDeque, time::Instant};

/// Different conditions for printing an update of the current progress
#[derive(Debug)]
pub enum ProgressUpdateCondition {
    /// Print whenever the current progress percentage has changed to a new multiple of the given value. Value is a percentage value in [0.0;100.0]
    OnPercentageChanged(f64),
    /// Print whenever the raw progress value has changed to a new multiple of the given value
    OnProgressChanged(f64),
}

/// Helper structure for tracking progress. Progress can be any number, integer or real
#[derive(Debug)]
pub struct ProgressTracker {
    current_progress: f64,
    target_progress: f64,
    update_condition: ProgressUpdateCondition,
    last_n_progresses: VecDeque<(f64, Instant)>,
}

impl ProgressTracker {
    const MAX_THROUGHPUTS_ENTRIES: usize = 32;

    pub fn new(target_progress: f64, update_condition: ProgressUpdateCondition) -> Self {
        if target_progress < 0.0 {
            panic!("ProgressTracker::new: target_progress must be a positive number!");
        }
        Self {
            current_progress: 0.0,
            target_progress,
            update_condition,
            last_n_progresses: VecDeque::new(),
        }
    }

    pub fn inc_progress(&mut self, increment: f64) {
        if increment < 0.0 {
            panic!("ProgressTracker::inc_progress: increment must be a positive number!");
        }
        if self.current_progress == self.target_progress {
            return;
        }

        let old_progress = self.current_progress;
        if self.current_progress + increment >= self.target_progress {
            self.current_progress = self.target_progress;
            let mean_throughput = self.calculate_throughput(old_progress, self.current_progress);
            self.print_progress(mean_throughput);
            return;
        }

        self.current_progress += increment;
        let mean_throughput = self.calculate_throughput(old_progress, self.current_progress);

        match self.update_condition {
            ProgressUpdateCondition::OnPercentageChanged(percentage_step) => {
                let old_percentage = old_progress / self.target_progress;
                let new_percentage = self.current_progress / self.target_progress;
                let old_percentage_steps = (old_percentage / percentage_step) as usize;
                let new_percentage_steps = (new_percentage / percentage_step) as usize;
                if new_percentage_steps > old_percentage_steps {
                    self.print_progress(mean_throughput);
                }
            }
            ProgressUpdateCondition::OnProgressChanged(progress_step) => {
                let old_steps = (old_progress / progress_step) as usize;
                let new_steps = (self.current_progress / progress_step) as usize;
                if new_steps > old_steps {
                    self.print_progress(mean_throughput);
                }
            }
        }
    }

    fn calculate_throughput(&mut self, old_progress: f64, new_progress: f64) -> Option<f64> {
        let now = Instant::now();
        if self.last_n_progresses.len() == Self::MAX_THROUGHPUTS_ENTRIES {
            self.last_n_progresses.pop_front();
        }
        self.last_n_progresses.push_back((new_progress, now));

        if self.last_n_progresses.len() < 2 {
            return None;
        }

        // Oldest progress is last_n_progresses.front()
        let oldest_progress = self.last_n_progresses.front().unwrap();
        let newest_progress = self.last_n_progresses.back().unwrap();
        let delta_time = (newest_progress.1).duration_since(oldest_progress.1);
        let delta_progress = newest_progress.0 - oldest_progress.0;
        Some(delta_progress / delta_time.as_secs_f64())
    }

    fn print_progress(&mut self, mean_throughput: Option<f64>) {
        let progress_percentage =
            100.0 * self.current_progress as f64 / self.target_progress as f64;

        match mean_throughput {
            Some(throughput) => {
                let remaining_progress = self.target_progress - self.current_progress;
                let etr_seconds = remaining_progress / throughput;
                eprintln!("{:.2}% [ETA: {:.0}s]", progress_percentage, etr_seconds);
            }
            None => eprintln!("{:.2}%", progress_percentage),
        }
    }
}
