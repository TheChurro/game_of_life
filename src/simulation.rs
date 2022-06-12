use bevy::{math::IVec2, prelude::Component, utils::HashMap};

use crate::tiling::{TileShape, Tiling, TilingKind};

#[derive(Component)]
pub struct SimulationState {
    pub tiling: Tiling,
    num_states: usize,
    states: HashMap<TileShape, Vec<StateRules>>,
    index_to_state: HashMap<IVec2, SimulationCellState>,
    manual_sets: HashMap<IVec2, u32>,
    pending_sets: HashMap<IVec2, u32>,
}

struct StateRule {
    min: u32,
    max: u32,
    neighbor_states_to_count: Vec<u32>,
    output: u32,
}

struct StateRules {
    default_state: u32,
    rules: Vec<StateRule>,
}

struct SimulationCellState {
    state: u32,
    neighbors_in_state: Vec<u32>,
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
                        rules: vec![StateRule {
                            min: 2,
                            max: 2,
                            neighbor_states_to_count: vec![1],
                            output: 1,
                        }],
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
                            min: 2,
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
                            max: 2,
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
            states,
            num_states,
            index_to_state: Default::default(),
            manual_sets: Default::default(),
            pending_sets: Default::default(),
        }
    }

    pub fn set_at(&mut self, index: IVec2, new_state: u32) {
        self.manual_sets
            .insert(self.tiling.adjust_index(index), new_state);
    }

    pub fn get_at(&self, index: IVec2) -> u32 {
        match self.index_to_state.get(&index) {
            Some(state) => state.state,
            None => 0u32,
        }
    }

    pub fn process(&mut self, do_real_tick: bool) {
        // If we are doing a real tick, take in the value from the last process
        // step along with the usual normal values.
        if do_real_tick {
            for (key, value) in self.pending_sets.drain() {
                self.manual_sets.try_insert(key, value).ok();
            }
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

            // Once we have updated the target state, move to all neighbors and alert them that
            // we have replaced the old neighbor value with it's new value. If this results in
            // any sets for the next round, then store them in pending sets.
            for neighbor in neighbors {
                let neighbor_index = key + IVec2::from(*neighbor);
                let neighbor_shape = self.tiling.get_tile_at_index(neighbor_index).shape;
                let default_rules = Vec::new();
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
