use fastrand::Rng;

pub trait Schedule {
    fn next_interval(
        &mut self,
        previous_interval: u16,
        was_correct: Option<bool>,
        interval_modifier: u16,
    ) -> u16;
}

/// The reduction factor applied to the next interval when the card was answered incorrectly.
const WRONG_ANSWER_PENALTY: f64 = 0.7;

/// A [`Schedule`] which implements
/// [low-key Anki](https://refold.la/roadmap/stage-1/a/anki-setup/).
pub struct LowKeyAnki {
    rng: Rng,
}

impl Schedule for LowKeyAnki {
    fn next_interval(
        &mut self,
        previous_interval: u16,
        was_correct: Option<bool>,
        interval_modifier: u16,
    ) -> u16 {
        match previous_interval {
            // Newly added card answered correctly
            0 => 1,
            // First review. The wrong answer penalty isn't applied since it's rare to answer
            // incorrectly on the first review.
            1 => 4,
            _ => {
                let was_correct = was_correct.expect("previously answered the card");

                if was_correct {
                    // Previous answer was correct
                    let mut next =
                        (previous_interval as f64 * 2.5 * (interval_modifier as f64 / 100.0))
                            as u16;

                    let max_fuzz =
                        (previous_interval as f64 * self.fuzz_factor(previous_interval)) as u16;
                    let fuzz = self.rng.u16(0..=max_fuzz);

                    if self.rng.bool() {
                        next += fuzz;
                    } else {
                        next -= fuzz;
                    }

                    next
                } else {
                    // Previous answer was wrong
                    std::cmp::max(1, (previous_interval as f64 * WRONG_ANSWER_PENALTY) as u16)
                }
            }
        }
    }
}

impl LowKeyAnki {
    /// Creates a new schedule implementing low-key Anki.
    pub fn new() -> Self {
        Self { rng: Rng::new() }
    }

    /// Returns a value in (0, 1) used to randomize the next interval for a card. This prevents
    /// cards getting grouped together based on when they were added.
    fn fuzz_factor(&self, previous_interval: u16) -> f64 {
        if previous_interval < 7 {
            0.25
        } else if previous_interval < 30 {
            0.15
        } else {
            0.05
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_schedule() -> LowKeyAnki {
        LowKeyAnki {
            rng: Rng::with_seed(0),
        }
    }

    #[test]
    fn first_answer() {
        let mut schedule = new_schedule();

        let next = schedule.next_interval(0, None, 100);

        assert_eq!(next, 1);
    }

    #[test]
    fn first_review() {
        let mut schedule = new_schedule();

        let next = schedule.next_interval(1, Some(true), 100);

        assert_eq!(next, 4);
    }

    #[test]
    fn apply_wrong_penalty() {
        let mut schedule = new_schedule();

        let next = schedule.next_interval(50, Some(false), 100);

        assert_eq!(next, (50 as f64 * WRONG_ANSWER_PENALTY) as u16);
    }

    #[test]
    fn correct_answer() {
        let mut schedule = new_schedule();

        let next = schedule.next_interval(50, Some(true), 100);

        assert_eq!(next, 124);
    }

    #[test]
    fn increase_by_interval_modifier() {
        let mut schedule = new_schedule();

        let next = schedule.next_interval(50, Some(true), 200);

        assert_eq!(next, 249);
    }

    #[test]
    #[should_panic]
    fn expect_previous_answer_for_large_interval() {
        let mut schedule = new_schedule();

        schedule.next_interval(50, None, 200);
    }
}
