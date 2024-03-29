use libafl::{
    bolts::tuples::MatchFirstType, executors::ExitKind, inputs::Input, observers::ObserversTuple,
};
use std::ops::Range;

use crate::executor::QemuExecutor;

// TODO remove 'static when specialization will be stable
pub trait QemuHelper<I, S>: 'static
where
    I: Input,
{
    fn init<'a, H, OT, QT>(&self, _executor: &QemuExecutor<'a, H, I, OT, QT, S>)
    where
        H: FnMut(&I) -> ExitKind,
        OT: ObserversTuple<I, S>,
        QT: QemuHelperTuple<I, S>,
    {
    }

    fn pre_exec(&mut self, _input: &I) {}

    fn post_exec(&mut self, _input: &I) {}
}

pub trait QemuHelperTuple<I, S>: MatchFirstType
where
    I: Input,
{
    fn init_all<'a, H, OT, QT>(&self, executor: &QemuExecutor<'a, H, I, OT, QT, S>)
    where
        H: FnMut(&I) -> ExitKind,
        OT: ObserversTuple<I, S>,
        QT: QemuHelperTuple<I, S>;

    fn pre_exec_all(&mut self, input: &I);

    fn post_exec_all(&mut self, input: &I);
}

impl<I, S> QemuHelperTuple<I, S> for ()
where
    I: Input,
{
    fn init_all<'a, H, OT, QT>(&self, _executor: &QemuExecutor<'a, H, I, OT, QT, S>)
    where
        H: FnMut(&I) -> ExitKind,
        OT: ObserversTuple<I, S>,
        QT: QemuHelperTuple<I, S>,
    {
    }

    fn pre_exec_all(&mut self, _input: &I) {}

    fn post_exec_all(&mut self, _input: &I) {}
}

impl<Head, Tail, I, S> QemuHelperTuple<I, S> for (Head, Tail)
where
    Head: QemuHelper<I, S>,
    Tail: QemuHelperTuple<I, S>,
    I: Input,
{
    fn init_all<'a, H, OT, QT>(&self, executor: &QemuExecutor<'a, H, I, OT, QT, S>)
    where
        H: FnMut(&I) -> ExitKind,
        OT: ObserversTuple<I, S>,
        QT: QemuHelperTuple<I, S>,
    {
        self.0.init(executor);
        self.1.init_all(executor);
    }

    fn pre_exec_all(&mut self, input: &I) {
        self.0.pre_exec(input);
        self.1.pre_exec_all(input);
    }

    fn post_exec_all(&mut self, input: &I) {
        self.0.post_exec(input);
        self.1.post_exec_all(input);
    }
}

pub enum QemuInstrumentationFilter {
    AllowList(Vec<Range<u64>>),
    DenyList(Vec<Range<u64>>),
    None,
}

impl QemuInstrumentationFilter {
    #[must_use]
    pub fn allowed(&self, addr: u64) -> bool {
        match self {
            QemuInstrumentationFilter::AllowList(l) => {
                for rng in l {
                    if rng.contains(&addr) {
                        return true;
                    }
                }
                false
            }
            QemuInstrumentationFilter::DenyList(l) => {
                for rng in l {
                    if rng.contains(&addr) {
                        return false;
                    }
                }
                true
            }
            QemuInstrumentationFilter::None => true,
        }
    }
}
