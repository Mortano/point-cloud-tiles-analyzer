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
}

impl ProgressTracker {
    pub fn new(target_progress: f64, update_condition: ProgressUpdateCondition) -> Self {
        if target_progress < 0.0 {
            panic!("ProgressTracker::new: target_progress must be a positive number!");
        }
        Self {
            current_progress: 0.0,
            target_progress,
            update_condition,
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
            self.print_progress();
            return;
        }

        self.current_progress += increment;

        match self.update_condition {
            ProgressUpdateCondition::OnPercentageChanged(percentage_step) => {
                let old_percentage = old_progress / self.target_progress;
                let new_percentage = self.current_progress / self.target_progress;
                let old_percentage_steps = (old_percentage / percentage_step) as usize;
                let new_percentage_steps = (new_percentage / percentage_step) as usize;
                if new_percentage_steps > old_percentage_steps {
                    self.print_progress();
                }
            }
            ProgressUpdateCondition::OnProgressChanged(progress_step) => {
                let old_steps = (old_progress / progress_step) as usize;
                let new_steps = (self.current_progress / progress_step) as usize;
                if new_steps > old_steps {
                    self.print_progress();
                }
            }
        }
    }

    fn print_progress(&self) {
        let progress_percentage =
            100.0 * self.current_progress as f64 / self.target_progress as f64;
        eprintln!("{:.2}%", progress_percentage);
    }
}
