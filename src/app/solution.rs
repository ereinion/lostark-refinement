use arrayvec::ArrayVec;
use fnv::FnvHashMap;
use rand::prelude::*;

use super::{chance::Chance, widgets::GameState};

#[derive(Debug, Clone, Copy)]
pub(super) struct Answer {
    pub(super) index: usize,
    pub(super) score: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct State {
    pub(super) chance: Chance,
    pub(super) remaining: [u8; 3],
}

impl From<&GameState> for State {
    fn from(gs: &GameState) -> Self {
        let num_slots = gs.num_slots();
        Self {
            chance: gs.chance(),
            remaining: [
                num_slots - gs.row(0).len() as u8,
                num_slots - gs.row(1).len() as u8,
                num_slots - gs.row(2).len() as u8,
            ],
        }
    }
}

impl State {
    fn available_choices(&self) -> ArrayVec<usize, 3> {
        let mut out = ArrayVec::new();
        for i in 0..3 {
            if self.remaining[i] > 0 {
                out.push(i);
            }
        }
        out
    }

    fn transition(&self, choice: usize) -> (Self, Self) {
        assert!(self.remaining[choice] > 0);
        let mut success = *self;
        success.remaining[choice] -= 1;
        let mut fail = success;

        success.chance.down();
        fail.chance.up();

        (success, fail)
    }

    fn update(&mut self, choice: usize, rng: &mut ThreadRng) -> bool {
        assert!(self.remaining[choice] > 0);
        self.remaining[choice] -= 1;
        if rng.gen::<f64>() < self.chance.as_f64() {
            self.chance.down();
            true
        } else {
            self.chance.up();
            false
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(super) struct Scoring {
    pub(super) success: [f64; 3],
    pub(super) fail: [f64; 3],
}

impl Scoring {
    pub(super) fn eval(&self, scores: [u8; 3], count: u8) -> f64 {
        self.success[0] * f64::from(scores[0])
            + self.success[1] * f64::from(scores[1])
            + self.success[2] * f64::from(scores[2])
            + self.fail[0] * (f64::from(count) - f64::from(scores[0]))
            + self.fail[1] * (f64::from(count) - f64::from(scores[1]))
            + self.fail[2] * (f64::from(count) - f64::from(scores[2]))
    }
}

#[derive(Debug)]
pub(super) struct Solution {
    scoring: Scoring,
    optimal: FnvHashMap<State, Answer>,
    count: u8,
}

impl Solution {
    pub(super) fn build(scoring: Scoring, count: u8) -> Self {
        let mut this = Self {
            scoring,
            optimal: FnvHashMap::default(),
            count,
        };
        this.build_impl();
        this
    }

    pub(super) fn num_states(&self) -> usize {
        self.optimal.len()
    }

    fn build_impl(&mut self) {
        let mut remaining = [0, 0, 0];
        loop {
            for chance in ALL_CHANCES {
                let state = State {
                    chance,
                    remaining: [remaining[0], remaining[1], remaining[2]],
                };
                let available_choices = state.available_choices();
                if available_choices.is_empty() {
                    continue;
                }

                let mut scores = ArrayVec::<_, 3>::new();
                let prob_success = state.chance.as_f64();
                let prob_fail = 1.0 - prob_success;

                for index in available_choices {
                    let (success_state, fail_state) = state.transition(index);
                    let success_score = self.lookup(&success_state).map(|a| a.score).unwrap_or(0.0);
                    let fail_score = self.lookup(&fail_state).map(|a| a.score).unwrap_or(0.0);

                    let score = prob_success * (self.scoring.success[index] + success_score)
                        + prob_fail * (self.scoring.fail[index] + fail_score);

                    scores.push(Answer { index, score });
                }

                scores.sort_by(|a, b| {
                    a.score
                        .partial_cmp(&b.score)
                        .unwrap()
                        .then(b.index.cmp(&a.index))
                });
                self.optimal.insert(state, scores.pop().unwrap());
            }

            // odometer to next `remaining`; this is ugly but meh
            remaining[2] += 1;
            if remaining[2] > self.count {
                remaining[2] = 0;
                remaining[1] += 1;
                if remaining[1] > self.count {
                    remaining[1] = 0;
                    remaining[0] += 1;
                    if remaining[0] > self.count {
                        break;
                    }
                }
            }
        }
    }

    fn lookup(&self, state: &State) -> Option<Answer> {
        if let Some(answer) = self.optimal.get(&state) {
            return Some(*answer);
        }
        assert!(
            state.available_choices().is_empty(),
            "bad lookup: {:?}",
            state
        );
        None
    }

    pub(super) fn optimal_choice(&self, state: &GameState) -> Option<Answer> {
        let state = State::from(state);
        self.lookup(&state)
    }

    pub(super) fn simulate_once(&self, start: &GameState, rng: &mut ThreadRng) -> [u8; 3] {
        assert_eq!(self.count, start.num_slots());
        let mut state = State::from(start);
        let mut scores = [
            start.row(0).iter().filter(|&&x| x).count() as u8,
            start.row(1).iter().filter(|&&x| x).count() as u8,
            start.row(2).iter().filter(|&&x| x).count() as u8,
        ];

        while !state.available_choices().is_empty() {
            // lookup is guaranteed to succeed as long as we have at least one
            // available choice
            let best = self.lookup(&state).unwrap();
            let success = state.update(best.index, rng);
            if success {
                scores[best.index] += 1;
            }
        }

        scores
    }

    pub(super) fn eval_result(&self, result: [u8; 3]) -> f64 {
        self.scoring.eval(result, self.count)
    }
}

const ALL_CHANCES: [Chance; 6] = [
    Chance::TwentyFive,
    Chance::ThirtyFive,
    Chance::FourtyFive,
    Chance::FiftyFive,
    Chance::SixtyFive,
    Chance::SeventyFive,
];
