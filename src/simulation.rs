use bevy::{math::IVec2, prelude::Component, utils::HashMap};

use crate::tiling::{EquilateralDirection, RightTriangleRotation, TileShape, Tiling, TilingKind};

#[derive(Component)]
pub struct SimulationState {
    pub tiling: Tiling,
    pub run_every: u32,
    pub step: u32,
    time_since_last_update: u32,
    pub num_states: usize,
    states: HashMap<TileShape, Vec<StateRules>>,
    index_to_state: HashMap<IVec2, SimulationCellState>,
    manual_sets: HashMap<IVec2, u32>,
    pending_sets: HashMap<IVec2, u32>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RuleUpdateTarget {
    DefaultValue,
    MinValue,
    MaxValue,
    ToggleCount,
    ResultValue,
}

#[derive(Clone)]
pub struct StateRule {
    pub min: u32,
    pub max: u32,
    pub neighbor_states_to_count: Vec<u32>,
    pub output: u32,
}

#[derive(Clone)]
pub struct StateRules {
    pub default_state: u32,
    pub rules: Vec<StateRule>,
}

struct SimulationCellState {
    pub state: u32,
    pub neighbors_in_state: Vec<u32>,
}

impl SimulationCellState {
    pub fn new(state: u32, num_neighbors: u32, states: usize) -> Self {
        let mut neighbors_in_state = vec![0; states.max(1)];
        neighbors_in_state[0] = num_neighbors;
        Self {
            state,
            neighbors_in_state,
        }
    }

    // Apply a change
    pub fn apply_change(
        &mut self,
        replaced_state: u32,
        new_state: u32,
        rules: &Vec<StateRules>,
    ) -> Option<u32> {
        if replaced_state as usize >= self.neighbors_in_state.len()
            || new_state as usize >= self.neighbors_in_state.len()
        {
            panic!(
                "Tried to replace {} with {} however we only have {} states",
                replaced_state,
                new_state,
                self.neighbors_in_state.len()
            );
        }
        if self.state as usize >= rules.len() {
            panic!("We do not have a rule registered for this state!");
        }
        self.neighbors_in_state[replaced_state as usize] -= 1;
        self.neighbors_in_state[new_state as usize] += 1;

        self.evaluate(rules)
    }

    fn evaluate(&self, rules: &Vec<StateRules>) -> Option<u32> {
        let mut final_value = rules[self.state as usize].default_state;
        for rule in &rules[self.state as usize].rules {
            let count = rule
                .neighbor_states_to_count
                .iter()
                .fold(0u32, |value, state| {
                    value + self.neighbors_in_state[*state as usize]
                });
            if rule.min <= count && count <= rule.max {
                final_value = rule.output;
                break;
            }
        }
        if final_value == self.state {
            None
        } else {
            Some(final_value)
        }
    }
}

fn get_default_rules_for_tiling(kind: TilingKind) -> HashMap<TileShape, Vec<StateRules>> {
    let mut map: HashMap<TileShape, Vec<StateRules>> = Default::default();
    match kind {
        TilingKind::Square => {
            map.insert(
                TileShape::Square,
                vec![
                    StateRules {
                        default_state: 0,
                        rules: vec![StateRule {
                            min: 3,
                            max: 3,
                            neighbor_states_to_count: vec![1],
                            output: 1,
                        }],
                    },
                    StateRules {
                        default_state: 0,
                        rules: vec![StateRule {
                            min: 2,
                            max: 3,
                            neighbor_states_to_count: vec![1],
                            output: 1,
                        }],
                    },
                ],
            );
        }
        TilingKind::Hexagonal => {
            map.insert(
                TileShape::Hexagon,
                vec![
                    StateRules {
                        default_state: 0,
                        rules: vec![StateRule {
                            min: 2,
                            max: 2,
                            neighbor_states_to_count: vec![1],
                            output: 1,
                        }],
                    },
                    StateRules {
                        default_state: 0,
                        rules: vec![
                            StateRule {
                                min: 3,
                                max: 3,
                                neighbor_states_to_count: vec![1],
                                output: 1,
                            },
                            StateRule {
                                min: 5,
                                max: 5,
                                neighbor_states_to_count: vec![1],
                                output: 1,
                            },
                        ],
                    },
                ],
            );
        }
        TilingKind::OctagonAndSquare => {
            map.insert(
                TileShape::Octagon,
                vec![
                    StateRules {
                        default_state: 0,
                        rules: vec![StateRule {
                            min: 3,
                            max: 3,
                            neighbor_states_to_count: vec![1],
                            output: 1,
                        }],
                    },
                    StateRules {
                        default_state: 0,
                        rules: vec![StateRule {
                            min: 3,
                            max: 3,
                            neighbor_states_to_count: vec![1],
                            output: 1,
                        }],
                    },
                ],
            );
            map.insert(
                TileShape::Square,
                vec![
                    StateRules {
                        default_state: 0,
                        rules: vec![StateRule {
                            min: 2,
                            max: 3,
                            neighbor_states_to_count: vec![1],
                            output: 1,
                        }],
                    },
                    StateRules {
                        default_state: 0,
                        rules: vec![StateRule {
                            min: 1,
                            max: 2,
                            neighbor_states_to_count: vec![1],
                            output: 1,
                        }],
                    },
                ],
            );
        }
        TilingKind::EquilateralTriangular => {
            for direction in [EquilateralDirection::Up, EquilateralDirection::Down] {
                map.insert(
                    TileShape::EquilateralTriangle(direction),
                    vec![
                        StateRules {
                            default_state: 0,
                            rules: vec![StateRule {
                                min: 3,
                                max: 3,
                                neighbor_states_to_count: vec![1],
                                output: 1,
                            }],
                        },
                        StateRules {
                            default_state: 0,
                            rules: vec![StateRule {
                                min: 3,
                                max: 3,
                                neighbor_states_to_count: vec![1],
                                output: 1,
                            }],
                        },
                    ],
                );
            }
        }
        TilingKind::RightTriangular => {
            for rotation in [
                RightTriangleRotation::Zero,
                RightTriangleRotation::One,
                RightTriangleRotation::Two,
                RightTriangleRotation::Three,
            ] {
                map.insert(
                    TileShape::RightTriangle(rotation),
                    vec![
                        StateRules {
                            default_state: 0,
                            rules: vec![StateRule {
                                min: 3,
                                max: 3,
                                neighbor_states_to_count: vec![1],
                                output: 1,
                            }],
                        },
                        StateRules {
                            default_state: 0,
                            rules: vec![StateRule {
                                min: 3,
                                max: 3,
                                neighbor_states_to_count: vec![1],
                                output: 1,
                            }],
                        },
                    ],
                );
            }
        }
    }
    map
}

impl SimulationState {
    pub fn new(tiling: Tiling) -> Self {
        let states = get_default_rules_for_tiling(tiling.kind);
        let num_states = (&states)
            .iter()
            .fold(1usize, |max, (_, states)| max.max(states.len()));
        Self {
            tiling,
            run_every: 0,
            step: 0,
            time_since_last_update: 0,
            states,
            num_states,
            index_to_state: Default::default(),
            manual_sets: Default::default(),
            pending_sets: Default::default(),
        }
    }

    pub fn get_shapes(&self) -> Vec<TileShape> {
        self.states.keys().cloned().collect()
    }

    pub fn get_num_states_for_shape(&self, shape: TileShape) -> u32 {
        self.states.get(&shape).map(|rules| rules.len() as u32).unwrap_or(0)
    }

    pub fn clone_rules_for_shape(&self, shape: TileShape) -> Vec<StateRules> {
        self.states.get(&shape).cloned().unwrap_or_default()
    }

    pub fn set_rule_value(
        &mut self,
        shape: TileShape,
        state: u32,
        rule_number: usize,
        value: u32,
        target: RuleUpdateTarget,
    ) {
        if let Some(rules) = self.states.get_mut(&shape) {
            if state as usize >= rules.len() {
                return;
            }

            if let Some(rules) = rules.get_mut(state as usize) {
                if target == RuleUpdateTarget::DefaultValue {
                    rules.default_state = value;
                    return;
                }
                if let Some(rule) = rules.rules.get_mut(rule_number) {
                    match target {
                        RuleUpdateTarget::MinValue => {
                            rule.min = value;
                        }
                        RuleUpdateTarget::MaxValue => {
                            rule.max = value;
                        }
                        RuleUpdateTarget::ToggleCount => {
                            if let Some((index, _)) = rule
                                .neighbor_states_to_count
                                .iter()
                                .enumerate()
                                .filter(|(_, state)| **state == value)
                                .next()
                            {
                                rule.neighbor_states_to_count.remove(index);
                            } else {
                                rule.neighbor_states_to_count.push(value);
                            }
                        }
                        RuleUpdateTarget::ResultValue => {
                            rule.output = value;
                        }
                        RuleUpdateTarget::DefaultValue => {}
                    }
                }
            }
        }

        self.re_evaluate_cells();
    }

    pub fn add_state(&mut self, shape: TileShape) {
        if let Some(rules) = self.states.get_mut(&shape) {
            rules.push(StateRules {
                default_state: 0,
                rules: Vec::new(),
            });
            if self.num_states < rules.len() {
                for _ in self.num_states..rules.len() {
                    for state in self.index_to_state.values_mut() {
                        state.neighbors_in_state.push(0);
                    }
                }
                self.num_states = rules.len();
            }
        }
    }

    pub fn add_rule(&mut self, shape: TileShape, state: u32) {
        if let Some(rules) = self.states.get_mut(&shape) {
            if let Some(rule) = rules.get_mut(state as usize) {
                rule.rules.push(StateRule {
                    min: 0,
                    max: 0,
                    neighbor_states_to_count: Vec::new(),
                    output: 0,
                })
            }
        }

        self.re_evaluate_cells();
    }

    fn re_evaluate_cells(&mut self) {
        self.pending_sets.clear();

        for (index, state) in &self.index_to_state {
            if let Some(rules) = self
                .states
                .get(&self.tiling.get_tile_at_index(*index).shape)
            {
                if let Some(next_value) = state.evaluate(rules) {
                    self.pending_sets.insert(*index, next_value);
                }
            }
        }
    }

    pub fn set_at(&mut self, index: IVec2, new_state: u32) {
        self.manual_sets
            .insert(self.tiling.adjust_index(index), new_state);
    }

    pub fn get_at(&self, index: IVec2) -> u32 {
        match self.index_to_state.get(&self.tiling.adjust_index(index)) {
            Some(state) => state.state,
            None => 0u32,
        }
    }

    pub fn get_neighbor_count(&self, index: IVec2, neighbor_state: u32) -> u32 {
        match self.index_to_state.get(&self.tiling.adjust_index(index)) {
            Some(state) => state.neighbors_in_state[neighbor_state as usize],
            None => 0u32,
        }
    }

    pub fn get_pending(&self, index: IVec2) -> u32 {
        match self.manual_sets.get(&self.tiling.adjust_index(index)) {
            Some(value) => *value,
            None => match self.pending_sets.get(&self.tiling.adjust_index(index)) {
                Some(value) => *value,
                None => self.get_at(index),
            },
        }
    }

    pub fn process(&mut self) {
        // If we are doing a real tick, take in the value from the last process
        // step along with the usual normal values.
        if self.step > 0 {
            self.step -= 1;
            for (key, value) in self.pending_sets.drain() {
                self.manual_sets.try_insert(key, value).ok();
            }
        } else if self.run_every != 0 {
            if self.time_since_last_update == 0 {
                for (key, value) in self.pending_sets.drain() {
                    self.manual_sets.try_insert(key, value).ok();
                }
                self.time_since_last_update = self.run_every;
            }
            self.time_since_last_update -= 1;
        }

        // Iterate all sets that we need to process and update their state
        for (key, value) in self.manual_sets.drain() {
            let neighbors = self.tiling.get_neighbors(key);
            let old_value = if let Some(state) = self.index_to_state.get_mut(&key) {
                let old_value = state.state;
                state.state = value;
                old_value
            } else {
                self.index_to_state.insert(
                    key,
                    SimulationCellState::new(value, neighbors.len() as u32, self.num_states),
                );
                0u32
            };

            // Determine if after updating our state we need to change our state in the next step.
            let default_rules = Vec::new();
            let shape = self.tiling.get_tile_at_index(key).shape;
            let rules = self.states.get(&shape).unwrap_or(&default_rules);
            if let Some(state) = self.index_to_state.get_mut(&key) {
                if let Some(new_state) = state.evaluate(rules) {
                    self.pending_sets.insert(key, new_state);
                } else {
                    self.pending_sets.remove(&key);
                }
            }

            // Once we have updated the target state, move to all neighbors and alert them that
            // we have replaced the old neighbor value with it's new value. If this results in
            // any sets for the next round, then store them in pending sets.
            for neighbor in neighbors {
                let neighbor_index = self.tiling.adjust_index(key + IVec2::from(*neighbor));
                let neighbor_shape = self.tiling.get_tile_at_index(neighbor_index).shape;
                let neighbor_rules = self.states.get(&neighbor_shape).unwrap_or(&default_rules);
                if let Some(state) = self.index_to_state.get_mut(&neighbor_index) {
                    if let Some(new_state) = state.apply_change(old_value, value, neighbor_rules) {
                        self.pending_sets.insert(neighbor_index, new_state);
                    } else {
                        self.pending_sets.remove(&neighbor_index);
                    }
                } else {
                    let mut state = SimulationCellState::new(
                        0u32,
                        self.tiling.get_neighbors(neighbor_index).len() as u32,
                        self.num_states,
                    );
                    if let Some(new_state) = state.apply_change(old_value, value, neighbor_rules) {
                        self.pending_sets.insert(neighbor_index, new_state);
                    } else {
                        self.pending_sets.remove(&neighbor_index);
                    }
                    self.index_to_state.insert(neighbor_index, state);
                };
            }
        }
    }
}
