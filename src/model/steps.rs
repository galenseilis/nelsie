use crate::model::Length;
use pyo3::PyResult;
use std::collections::Bound::Included;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Debug, Display, Write};
use std::hash::Hash;
use std::ops::Bound::Unbounded;
use std::str::FromStr;

pub type Step = u32;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StepValue<T: Debug> {
    Const(T),
    Steps(BTreeMap<Step, T>),
}

impl<T: Debug> StepValue<T> {
    pub fn from_btree(tree: BTreeMap<Step, T>) -> Self {
        StepValue::Steps(tree)
    }
}

impl<T: Debug> StepValue<T> {
    pub fn new_const(value: T) -> Self {
        StepValue::Const(value)
    }

    pub fn new_map(value: BTreeMap<Step, T>) -> Self {
        StepValue::Steps(value)
    }

    pub fn at_step(&self, step: Step) -> &T {
        assert!(step > 0);
        match self {
            StepValue::Const(v) => v,
            StepValue::Steps(steps) => steps
                .range((Unbounded, Included(&step)))
                .next_back()
                .map(|(_, v)| v)
                .unwrap_or_else(|| panic!("Invalid step")),
        }
    }

    pub fn into_value(self) -> T {
        match self {
            StepValue::Const(v) => v,
            StepValue::Steps(_) => panic!("Not a const value"),
        }
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        match self {
            StepValue::Const(v) => itertools::Either::Left(std::iter::once(v)),
            StepValue::Steps(v) => itertools::Either::Right(v.values()),
        }
    }

    pub fn key_steps(&self) -> impl Iterator<Item = Step> + '_ {
        match self {
            StepValue::Const(_) => itertools::Either::Left(std::iter::empty()),
            StepValue::Steps(v) => itertools::Either::Right(v.keys().copied()),
        }
    }

    pub fn map<S: Debug, F: FnMut(T) -> S>(self, mut f: F) -> StepValue<S> {
        match self {
            StepValue::Const(v) => StepValue::Const(f(v)),
            StepValue::Steps(v) => {
                StepValue::Steps(v.into_iter().map(|(k, v)| (k, f(v))).collect())
            }
        }
    }

    pub fn map_ref<S: Debug, F: FnMut(&T) -> S>(&self, mut f: F) -> StepValue<S> {
        match self {
            StepValue::Const(v) => StepValue::Const(f(&v)),
            StepValue::Steps(v) => StepValue::Steps(v.iter().map(|(k, v)| (*k, f(v))).collect()),
        }
    }

    pub fn merge<S: Debug, R: Debug, F: FnMut(&T, &S) -> R>(
        &self,
        other: &StepValue<S>,
        mut f: F,
    ) -> StepValue<R> {
        match (self, other) {
            (StepValue::Const(v1), StepValue::Const(v2)) => StepValue::Const(f(v1, v2)),
            (StepValue::Steps(v1), StepValue::Const(v2)) => {
                StepValue::Steps(v1.iter().map(|(k, v)| (*k, f(v, v2))).collect())
            }
            (StepValue::Const(v1), StepValue::Steps(v2)) => {
                StepValue::Steps(v2.iter().map(|(k, v)| (*k, f(v1, v))).collect())
            }
            (StepValue::Steps(v1), StepValue::Steps(v2)) => {
                todo!()
            }
        }
    }
}
//
// pub(crate) fn zip_step_values<T: Debug + Clone>(
//     mut values: Vec<StepValue<T>>,
// ) -> StepValue<Vec<T>> {
//     if values.len() == 1 {
//         return values.pop().unwrap().map(|v| vec![v]);
//     }
//     let steps: BTreeSet<Step> = values.iter().map(|x| x.key_steps()).flatten().collect();
//     if steps.is_empty() {
//         return StepValue::new_const(values.into_iter().map(|v| v.into_value()).collect());
//     }
//     let map: BTreeMap<Step, Vec<T>> = steps
//         .into_iter()
//         .map(|step| {
//             (
//                 step,
//                 values
//                     .iter()
//                     .map(|v| v.at_step(step).clone())
//                     .collect::<Vec<T>>(),
//             )
//         })
//         .collect();
//     StepValue::new_map(map)
// }
