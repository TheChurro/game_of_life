use std::ops::{BitOr, BitAnd};

use super::orientations::GeomOrientation;


#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GeometryHandle {
    pub index: usize,
    pub orientation: GeomOrientation,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct GeometryHandleSetEntry {
    index: usize,
    orientations: usize,
}

#[derive(Clone)]
pub struct GeometryHandleSet {
    entries: Vec<GeometryHandleSetEntry>,
    max_rotations: usize,
}

impl GeometryHandleSet {
    pub fn new(max_rotations: usize) -> Self {
        Self { entries: Vec::new(), max_rotations }
    }

    fn get_location_for_index(&self, index: usize) -> usize {
        let mut range_min = 0;
        let mut range_max = self.entries.len();
        while range_min < range_max {
            let at = (range_max + range_min) / 2;
            match index.cmp(&self.entries[at].index) {
                std::cmp::Ordering::Less => {
                    range_max = at;
                },
                std::cmp::Ordering::Equal => {
                    return at;
                },
                std::cmp::Ordering::Greater => {
                    range_min = at + 1;
                },
            }
        }
        range_min
    }

    pub fn insert(&mut self, handle: GeometryHandle) {
        let location = self.get_location_for_index(handle.index);
        if location < self.entries.len() && self.entries[location].index == handle.index {
            self.entries[location].orientations |= handle.orientation.to_bits();
        } else {
            self.entries.insert(location, GeometryHandleSetEntry {
                index: handle.index,
                orientations: handle.orientation.to_bits()
            });
        }
    }

    pub fn contains(&self, handle: GeometryHandle) -> bool {
        let location = self.get_location_for_index(handle.index);
        if location < self.entries.len() && self.entries[location].index == handle.index {
            self.entries[location].orientations & handle.orientation.to_bits() != 0
        } else {
            false
        }
    }

    /// Compute the union of a number of GeometryHandleSets.
    pub fn union<'a, I: IntoIterator<Item=&'a GeometryHandleSet>>(sets: I) -> GeometryHandleSet {
        let sets = sets.into_iter().collect::<Vec<_>>();
        let mut new_entries = Vec::new();
        let mut max_rotations = 0;
        let mut last_min_index = None;
        let mut locations_at = vec![0; sets.len()];

        loop {
            let mut min_index = usize::MAX;
            let mut min_index_orientations = 0;
            for (i, set) in sets.iter().enumerate() {
                max_rotations = max_rotations.max(set.max_rotations);

                // Check to see if last iteration we consumed the entries from this set
                // or if we have already taken all the entries from this set.
                if locations_at[i] >= set.entries.len() { continue; }
                if Some(set.entries[locations_at[i]].index) == last_min_index {
                    locations_at[i] += 1;
                    if locations_at[i] >= set.entries.len() { continue; }
                }

                let entry = &set.entries[locations_at[i]];
                match entry.index.cmp(&min_index) {
                    std::cmp::Ordering::Less => {
                        min_index = entry.index;
                        min_index_orientations = entry.orientations;
                    },
                    std::cmp::Ordering::Equal => {
                        min_index_orientations |= entry.orientations;
                    },
                    std::cmp::Ordering::Greater => {},
                }
            }

            if min_index != usize::MAX {
                new_entries.push(GeometryHandleSetEntry { index: min_index, orientations: min_index_orientations });
                last_min_index = Some(min_index);
            } else {
                break;
            }
        }


        Self { entries: new_entries, max_rotations }
    }

    /// Compute the intersection of a number of geometry handle sets.
    pub fn intersection<'a, I : IntoIterator<Item=&'a GeometryHandleSet>>(sets: I) -> GeometryHandleSet {
        let sets = sets.into_iter().collect::<Vec<_>>();
        // Early out for single set intersection
        if sets.len() == 1 {
            return GeometryHandleSet {
                entries: sets[0].entries.iter().cloned().collect(),
                max_rotations: sets[0].max_rotations,
            };
        }

        let mut new_entries = Vec::new();
        let mut max_rotations = 0;

        if sets.len() > 0 && sets[0].entries.len() > 0 {
            let mut current_index = sets[0].entries[0].index;
            let mut current_index_orientations = sets[0].entries[0].orientations;
            let mut last_incremented_at = 0;
            let mut at = 1;

            let mut locations = vec![0; sets.len()];

            loop {
                max_rotations = max_rotations.max(sets[at].max_rotations);
                if last_incremented_at == at {
                    new_entries.push(GeometryHandleSetEntry { index: current_index, orientations: current_index_orientations });
                    locations[at] += 1;
                    if locations[at] < sets[at].entries.len() {
                        current_index = sets[at].entries[locations[at]].index;
                        current_index_orientations = sets[at].entries[locations[at]].orientations;
                    } else {
                        break;
                    }
                }

                let set = &sets[at];
                while locations[at] < set.entries.len() {
                    match set.entries[locations[at]].index.cmp(&current_index) {
                        std::cmp::Ordering::Less => { locations[at] += 1; },
                        std::cmp::Ordering::Equal => {
                            current_index_orientations &= set.entries[locations[at]].orientations;
                            if current_index_orientations == 0 {
                                locations[at] += 1;
                            } else {
                                break;
                            }
                        },
                        std::cmp::Ordering::Greater => {
                            current_index = set.entries[locations[at]].index;
                            current_index_orientations = set.entries[locations[at]].orientations;
                            last_incremented_at = at;
                            break;
                        },
                    }
                }

                // Check to see if last iteration we consumed the entries from this set
                // or if we have already taken all the entries from this set.
                if locations[at] >= set.entries.len() { break; }
                at += 1;
                at %= sets.len();
            }
        }

        Self { entries: new_entries, max_rotations }
    }

    pub fn empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn unique_index_count(&self) -> usize {
        self.entries.len()
    }
}

impl BitOr for &GeometryHandleSet {
    type Output = GeometryHandleSet;

    fn bitor(self, rhs: Self) -> Self::Output {
        GeometryHandleSet::union(vec![self, &rhs].drain(..))
    }
}

impl BitAnd for &GeometryHandleSet {
    type Output = GeometryHandleSet;

    fn bitand(self, rhs: Self) -> Self::Output {
        GeometryHandleSet::intersection(vec![self, &rhs].drain(..))
    }
}

pub struct GeometryHandleSetIterator<'a> {
    set: &'a GeometryHandleSet,
    location: usize,
    orientation: usize,
}

impl<'a> IntoIterator for &'a GeometryHandleSet {
    type Item = GeometryHandle;

    type IntoIter = GeometryHandleSetIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        GeometryHandleSetIterator {
            set: self,
            location: 0,
            orientation: 0
        }
    }
}

impl<'a> Iterator for GeometryHandleSetIterator<'a> {
    type Item = GeometryHandle;

    fn next(&mut self) -> Option<Self::Item> {
        while self.location < self.set.entries.len() {
            while self.orientation < self.set.max_rotations {
                self.orientation += 1;
                if self.set.entries[self.location].orientations & (1 << (self.orientation - 1 )) != 0 {
                    return Some(GeometryHandle {
                        index: self.set.entries[self.location].index,
                        orientation: GeomOrientation::Standard { rotations: self.orientation - 1 }
                    })
                }
            }

            while self.orientation < 2 * self.set.max_rotations {
                self.orientation += 1;
                if self.set.entries[self.location].orientations & (1 << (usize::MAX - self.orientation + self.set.max_rotations )) != 0 {
                    return Some(GeometryHandle {
                        index: self.set.entries[self.location].index,
                        orientation: GeomOrientation::Flipped { rotations: self.orientation - 1 - self.set.max_rotations }
                    })
                }
            }

            self.orientation = 0;
            self.location += 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::visuals::geom::GeomOrientation;

    use super::{GeometryHandleSet, GeometryHandle, GeometryHandleSetEntry};

    #[test]
    fn insert() {
        let mut set = GeometryHandleSet::new(5);
        set.insert(GeometryHandle {
            index: 2,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });

        assert_eq!(set.entries, vec![GeometryHandleSetEntry { index: 2, orientations: 0x4 }]);

        set.insert(GeometryHandle {
            index: 2,
            orientation: GeomOrientation::Standard { rotations: 0 },
        });

        set.insert(GeometryHandle {
            index: 0,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });

        assert_eq!(set.entries, vec![
            GeometryHandleSetEntry { index: 0, orientations: 0x4 },
            GeometryHandleSetEntry { index: 2, orientations: 0x5 }
        ]);

        set.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });

        assert_eq!(set.entries, vec![
            GeometryHandleSetEntry { index: 0, orientations: 0x4 },
            GeometryHandleSetEntry { index: 1, orientations: 0x2 },
            GeometryHandleSetEntry { index: 2, orientations: 0x5 }
        ]);

        set.insert(GeometryHandle {
            index: 3,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });

        assert_eq!(set.entries, vec![
            GeometryHandleSetEntry { index: 0, orientations: 0x4 },
            GeometryHandleSetEntry { index: 1, orientations: 0x2 },
            GeometryHandleSetEntry { index: 2, orientations: 0x5 },
            GeometryHandleSetEntry { index: 3, orientations: 0x2 }
        ]);
    }

    #[test]
    fn contains() {
        let mut set = GeometryHandleSet::new(5);
        set.insert(GeometryHandle {
            index: 2,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });
        set.insert(GeometryHandle {
            index: 2,
            orientation: GeomOrientation::Standard { rotations: 0 },
        });
        set.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });
        set.insert(GeometryHandle {
            index: 3,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });

        assert!(set.contains(GeometryHandle {
            index: 2,
            orientation: GeomOrientation::Standard { rotations: 2 },
        }));
        assert!(set.contains(GeometryHandle {
            index: 3,
            orientation: GeomOrientation::Standard { rotations: 1 },
        }));
        assert!(!set.contains(GeometryHandle {
            index: 3,
            orientation: GeomOrientation::Standard { rotations: 2 },
        }));
        assert!(!set.contains(GeometryHandle {
            index: 4,
            orientation: GeomOrientation::Standard { rotations: 2 },
        }));
        assert!(!set.contains(GeometryHandle {
            index: 0,
            orientation: GeomOrientation::Standard { rotations: 2 },
        }));
    }

    #[test]
    fn union_disjoint() {
        let mut set0 = GeometryHandleSet::new(3);
        let mut set1 = GeometryHandleSet::new(5);

        set0.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });
        set0.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });
        set0.insert(GeometryHandle {
            index: 2,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });

        set1.insert(GeometryHandle {
            index: 3,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });

        let union = GeometryHandleSet::union([&set0, &set1]);
        assert_eq!(union.entries, vec![
            GeometryHandleSetEntry { index: 1, orientations: 0x6 },
            GeometryHandleSetEntry { index: 2, orientations: 0x4 },
            GeometryHandleSetEntry { index: 3, orientations: 0x2 },
        ]);
        assert_eq!(union.max_rotations, 5);
    }

    #[test]
    fn union_alternating_index() {
        let mut set0 = GeometryHandleSet::new(5);
        let mut set1 = GeometryHandleSet::new(5);
        let mut set2 = GeometryHandleSet::new(5);

        set0.insert(GeometryHandle {
            index: 2,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });
        set0.insert(GeometryHandle {
            index: 5,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });

        set1.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });
        set1.insert(GeometryHandle {
            index: 4,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });

        set2.insert(GeometryHandle {
            index: 3,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });

        let union = GeometryHandleSet::union([&set0, &set1, &set2]);
        assert_eq!(union.entries, vec![
            GeometryHandleSetEntry { index: 1, orientations: 0x2 },
            GeometryHandleSetEntry { index: 2, orientations: 0x4 },
            GeometryHandleSetEntry { index: 3, orientations: 0x4 },
            GeometryHandleSetEntry { index: 4, orientations: 0x2 },
            GeometryHandleSetEntry { index: 5, orientations: 0x4 },
        ]);
    }

    #[test]
    fn union_multiple_at_index() {
        let mut set0 = GeometryHandleSet::new(5);
        let mut set1 = GeometryHandleSet::new(5);
        let mut set2 = GeometryHandleSet::new(5);

        set0.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });
        set0.insert(GeometryHandle {
            index: 2,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });

        set1.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });
        set1.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });

        set2.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 0 },
        });

        let union = GeometryHandleSet::union([&set0, &set1, &set2]);
        assert_eq!(union.entries, vec![
            GeometryHandleSetEntry { index: 1, orientations: 0x7 },
            GeometryHandleSetEntry { index: 2, orientations: 0x4 },
        ]);
    }

    #[test]
    fn union_single() {
        let mut set = GeometryHandleSet::new(5);
        set.insert(GeometryHandle {
            index: 0,
            orientation: GeomOrientation::Flipped { rotations: 1 }
        });
        let union = GeometryHandleSet::union([&set]);
        assert_eq!(union.entries, set.entries);
        assert_eq!(union.max_rotations, set.max_rotations);
    }

    #[test]
    fn union_empty() {
        let empty = GeometryHandleSet::union(&[]);
        assert_eq!(empty.entries, vec![]);
        assert_eq!(empty.max_rotations, 0);
    }

    #[test]
    fn intersection() {
        let mut set0 = GeometryHandleSet::new(3);
        let mut set1 = GeometryHandleSet::new(5);

        set0.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });
        set0.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });
        set0.insert(GeometryHandle {
            index: 2,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });
        set0.insert(GeometryHandle {
            index: 2,
            orientation: GeomOrientation::Standard { rotations: 0 },
        });

        set1.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });
        set1.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });
        set1.insert(GeometryHandle {
            index: 2,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });
        set1.insert(GeometryHandle {
            index: 2,
            orientation: GeomOrientation::Standard { rotations: 0 },
        });

        let intersection = GeometryHandleSet::intersection([&set0, &set1]);
        assert_eq!(intersection.entries, vec![
            GeometryHandleSetEntry { index: 1, orientations: 0x6 },
            GeometryHandleSetEntry { index: 2, orientations: 0x1 },
        ]);
        assert_eq!(intersection.max_rotations, 5);
    }
    
    #[test]
    fn intersection_disjoint() {
        let mut set0 = GeometryHandleSet::new(3);
        let mut set1 = GeometryHandleSet::new(5);

        set0.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });
        set0.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });
        set0.insert(GeometryHandle {
            index: 2,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });

        set1.insert(GeometryHandle {
            index: 3,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });

        let intersection = GeometryHandleSet::intersection([&set0, &set1]);
        assert_eq!(intersection.entries, vec![
        ]);
        assert_eq!(intersection.max_rotations, 5);
    }

    #[test]
    fn intersection_alternating_index() {
        let mut set0 = GeometryHandleSet::new(5);
        let mut set1 = GeometryHandleSet::new(5);
        let mut set2 = GeometryHandleSet::new(5);

        set0.insert(GeometryHandle {
            index: 2,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });
        set0.insert(GeometryHandle {
            index: 3,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });
        set0.insert(GeometryHandle {
            index: 5,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });

        set1.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });
        set1.insert(GeometryHandle {
            index: 3,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });

        set2.insert(GeometryHandle {
            index: 3,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });
        set2.insert(GeometryHandle {
            index: 4,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });

        let intersection = GeometryHandleSet::intersection([&set0, &set1, &set2]);
        assert_eq!(intersection.entries, vec![
            GeometryHandleSetEntry { index: 3, orientations: 0x2 },
        ]);
    }

    #[test]
    fn intersection_multiple_at_index() {
        let mut set0 = GeometryHandleSet::new(5);
        let mut set1 = GeometryHandleSet::new(5);
        let mut set2 = GeometryHandleSet::new(5);

        set0.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });
        set0.insert(GeometryHandle {
            index: 2,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });

        set1.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 1 },
        });
        set1.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });

        set2.insert(GeometryHandle {
            index: 1,
            orientation: GeomOrientation::Standard { rotations: 2 },
        });

        let intersection = GeometryHandleSet::intersection([&set0, &set1, &set2]);
        assert_eq!(intersection.entries, vec![
            GeometryHandleSetEntry { index: 1, orientations: 0x4 },
        ]);
    }

    #[test]
    fn intersection_single() {
        let mut set = GeometryHandleSet::new(5);
        set.insert(GeometryHandle {
            index: 0,
            orientation: GeomOrientation::Flipped { rotations: 1 }
        });
        let union = GeometryHandleSet::intersection([&set]);
        assert_eq!(union.entries, set.entries);
        assert_eq!(union.max_rotations, set.max_rotations);
    }

    #[test]
    fn intersection_empty() {
        let empty = GeometryHandleSet::intersection(&[]);
        assert_eq!(empty.entries, vec![]);
        assert_eq!(empty.max_rotations, 0);
    }
}