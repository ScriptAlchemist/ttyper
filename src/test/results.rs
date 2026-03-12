use super::{is_missed_word_event, Test};

use crossterm::event::{KeyCode, KeyEvent};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::{cmp, fmt};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fraction {
    pub numerator: usize,
    pub denominator: usize,
}

impl Fraction {
    pub const fn new(numerator: usize, denominator: usize) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

impl From<Fraction> for f64 {
    fn from(f: Fraction) -> Self {
        f.numerator as f64 / f.denominator as f64
    }
}

impl cmp::Ord for Fraction {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        f64::from(*self).partial_cmp(&f64::from(*other)).unwrap()
    }
}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Fraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

pub struct TimingData {
    // Instead of storing WPM, we store CPS (clicks per second)
    pub overall_cps: f64,
    pub per_event: Vec<f64>,
    pub per_key: HashMap<KeyEvent, f64>,
}

pub struct AccuracyData {
    pub overall: Fraction,
    pub per_key: HashMap<KeyEvent, Fraction>,
}

pub struct Results {
    pub typed_text: String,
    pub timing: TimingData,
    pub accuracy: AccuracyData,
    pub missed_words: Vec<String>,
}

impl Results {
    pub fn plain_text_summary(&self) -> String {
        let mut summary = String::new();
        let adjusted_wpm = self.timing.overall_cps * 12.0 * f64::from(self.accuracy.overall);
        let raw_wpm = self.timing.overall_cps * 12.0;
        let overall_accuracy = f64::from(self.accuracy.overall) * 100.0;

        if !self.typed_text.is_empty() {
            let _ = writeln!(summary, "{}", self.typed_text);
            let _ = writeln!(summary);
        }
        let _ = writeln!(summary, "Results");
        let _ = writeln!(summary, "Adjusted WPM: {:.1}", adjusted_wpm);
        let _ = writeln!(summary, "Accuracy: {:.1}%", overall_accuracy);
        let _ = writeln!(summary, "Raw WPM: {:.1}", raw_wpm);
        let _ = writeln!(summary, "Correct Keypresses: {}", self.accuracy.overall);

        let mut worst_keys: Vec<(char, Fraction)> = self
            .accuracy
            .per_key
            .iter()
            .filter_map(|(key, acc)| match key.code {
                KeyCode::Char(character)
                    if *acc != Fraction::new(acc.denominator, acc.denominator) =>
                {
                    Some((character, *acc))
                }
                _ => None,
            })
            .collect();
        worst_keys.sort_unstable_by(|(left_char, left_acc), (right_char, right_acc)| {
            left_acc.cmp(right_acc).then(left_char.cmp(right_char))
        });

        if worst_keys.is_empty() {
            let _ = writeln!(summary, "Worst Keys: none");
        } else {
            let _ = writeln!(summary, "Worst Keys:");
            for (character, accuracy) in worst_keys.into_iter().take(5) {
                let _ = writeln!(
                    summary,
                    "- {} at {:.1}% accuracy",
                    character,
                    f64::from(accuracy) * 100.0
                );
            }
        }

        summary
    }
}

impl From<&Test> for Results {
    fn from(test: &Test) -> Self {
        let events: Vec<&super::TestEvent> =
            test.words.iter().flat_map(|w| w.events.iter()).collect();

        Self {
            typed_text: render_typed_text(test),
            timing: calc_timing(&events),
            accuracy: calc_accuracy(&events),
            missed_words: calc_missed_words(test),
        }
    }
}

fn render_typed_text(test: &Test) -> String {
    let separator = if test
        .words
        .iter()
        .any(|word| word.text.chars().any(char::is_whitespace))
    {
        "\n"
    } else {
        " "
    };

    test.words
        .iter()
        .filter(|word| !word.progress.is_empty())
        .map(|word| word.progress.clone())
        .collect::<Vec<_>>()
        .join(separator)
}

fn calc_timing(events: &[&super::TestEvent]) -> TimingData {
    let mut timing = TimingData {
        overall_cps: -1.0,
        per_event: Vec::new(),
        per_key: HashMap::new(),
    };

    // map of keys to a two-tuple (total time, clicks) for counting average
    let mut keys: HashMap<KeyEvent, (f64, usize)> = HashMap::new();

    for win in events.windows(2) {
        let event_dur = win[1]
            .time
            .checked_duration_since(win[0].time)
            .map(|d| d.as_secs_f64());

        if let Some(event_dur) = event_dur {
            timing.per_event.push(event_dur);

            let key = keys.entry(win[1].key).or_insert((0.0, 0));
            key.0 += event_dur;
            key.1 += 1;
        }
    }

    timing.per_key = keys
        .into_iter()
        .map(|(key, (total, count))| (key, total / count as f64))
        .collect();

    timing.overall_cps = timing.per_event.len() as f64 / timing.per_event.iter().sum::<f64>();

    timing
}

fn calc_accuracy(events: &[&super::TestEvent]) -> AccuracyData {
    let mut acc = AccuracyData {
        overall: Fraction::new(0, 0),
        per_key: HashMap::new(),
    };

    events
        .iter()
        .filter(|event| event.correct.is_some())
        .for_each(|event| {
            let key = acc
                .per_key
                .entry(event.key)
                .or_insert_with(|| Fraction::new(0, 0));

            acc.overall.denominator += 1;
            key.denominator += 1;

            if event.correct.unwrap() {
                acc.overall.numerator += 1;
                key.numerator += 1;
            }
        });

    acc
}

fn calc_missed_words(test: &Test) -> Vec<String> {
    test.words
        .iter()
        .filter(|word| word.events.iter().any(is_missed_word_event))
        .map(|word| word.text.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn plain_text_summary_lists_key_metrics() {
        let mut per_key = HashMap::new();
        per_key.insert(
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
            Fraction::new(1, 2),
        );
        per_key.insert(
            KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
            Fraction::new(3, 4),
        );
        per_key.insert(
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
            Fraction::new(1, 1),
        );

        let results = Results {
            typed_text: "alpha beta".into(),
            timing: TimingData {
                overall_cps: 5.0,
                per_event: vec![],
                per_key: HashMap::new(),
            },
            accuracy: AccuracyData {
                overall: Fraction::new(9, 10),
                per_key,
            },
            missed_words: vec!["alpha".into(), "beta".into()],
        };

        let summary = results.plain_text_summary();

        assert!(summary.starts_with("alpha beta\n\nResults\n"));
        assert!(summary.contains("Adjusted WPM: 54.0"));
        assert!(summary.contains("Accuracy: 90.0%"));
        assert!(summary.contains("Raw WPM: 60.0"));
        assert!(summary.contains("Correct Keypresses: 9/10"));
        assert!(summary.contains("- a at 50.0% accuracy"));
        assert!(summary.contains("- z at 75.0% accuracy"));
        assert!(!summary.contains("- x at 100.0% accuracy"));
        assert!(!summary.contains("Missed Words"));
    }

    #[test]
    fn render_typed_text_only_includes_progress() {
        let mut test = Test::new(
            vec!["alpha".into(), "beta".into(), "gamma".into()],
            true,
            false,
        );
        test.words[0].progress = "alpha".into();
        test.words[1].progress = "be".into();

        assert_eq!(render_typed_text(&test), "alpha be");
    }

    #[test]
    fn render_typed_text_uses_newlines_for_line_based_prompts() {
        let mut test = Test::new(
            vec![
                "first line".into(),
                "second line".into(),
                "third line".into(),
            ],
            true,
            false,
        );
        test.words[0].progress = "first".into();
        test.words[1].progress = "second".into();

        assert_eq!(render_typed_text(&test), "first\nsecond");
    }
}
