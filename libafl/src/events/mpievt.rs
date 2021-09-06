//! A very simple event manager, that just supports log outputs, but no multiprocessing

use crate::{events::{
    BrokerEventResult, Event, EventFirer, EventManager, EventManagerId, EventProcessor,
    EventRestarter, HasEventManagerId,
}, inputs::Input, stats::Stats, Error, ExecutionProcessor, EvaluatorObservers};
use alloc::{string::ToString, vec::Vec};
#[cfg(feature = "std")]
use core::{
    marker::PhantomData,
};

#[cfg(all(feature = "std", windows))]
use crate::bolts::os::startable_self;
#[cfg(all(feature = "std", unix))]
//use crate::bolts::os::{fork, ForkResult};
use crate::observers::ObserversTuple;
use crate::executors::{Executor, HasObservers};
use mpi::topology::{Communicator, Rank, SystemCommunicator};
use mpi::point_to_point::{Source, Destination};
use mpi::Threading;

/// A simple, single-threaded event manager that just logs
pub struct MPIEventManager<I, ST, OT, S>
where
    I: Input,
    ST: Stats,
    OT: ObserversTuple<I, S> + serde::de::DeserializeOwned,
{
    /// The stats
    stats: ST,
    /// The events that happened since the last handle_in_broke
    communicator: SystemCommunicator,
    processor_ct: Rank,
    rank: Rank,
    phantom: PhantomData<(I, OT, S)>,
}

/// Send events with MPI
impl<I, S, ST, OT> EventFirer<I, S> for MPIEventManager<I, ST, OT, S>
where
    I: Input,
    ST: Stats,
    OT: ObserversTuple<I, S> + serde::de::DeserializeOwned,
{
    fn fire(&mut self, _state: &mut S, event: Event<I>) -> Result<(), Error> {
        match Self::handle_in_broker(&mut self.stats, &event)? {
            // Send the event with MPI
            BrokerEventResult::Forward => {
                let serialized = postcard::to_allocvec(&event)?;
                for i in 0..self.processor_ct {
                    if i==self.rank { continue }

                    self.communicator.process_at_rank(i).send(&serialized[..]);
                }

            }
            BrokerEventResult::Handled => (),
        };
        Ok(())
    }
}

impl<I, S, ST, OT> EventRestarter<S> for MPIEventManager<I, ST, OT, S>
where
    I: Input,
    ST: Stats,
    OT: ObserversTuple<I, S> + serde::de::DeserializeOwned,
{
}

impl<E, I, S, ST, Z, OT> EventProcessor<E, I, S, Z> for MPIEventManager<I, ST, OT, S>
where
    I: Input,
    E: Executor<Self, I, S, Z> + HasObservers<I, OT, S>,
    ST: Stats,
    OT: ObserversTuple<I, S> + serde::de::DeserializeOwned,
    Z: ExecutionProcessor<I, OT, S> + EvaluatorObservers<I, OT, S>,
{
    fn process(
        &mut self,
        fuzzer: &mut Z,
        state: &mut S,
        executor: &mut E,
    ) -> Result<usize, Error> {

        let mut count = 0;
        loop{
            if let Some(_) = self.communicator.any_process().immediate_probe(){
            } else {
                break;
            }

            let event_bytes: Vec<u8> = self.communicator.any_process().receive_vec().0;
            let event: Event<I> = postcard::from_bytes(&event_bytes)?;

            self.handle_in_client(fuzzer, executor, state, 0, event)?;
            count += 1;
        }

        Ok(count)
    }
}

impl<E, I, S, ST, Z, OT> EventManager<E, I, S, Z> for MPIEventManager<I, ST, OT, S>
where
    I: Input,
    E: Executor<Self, I, S, Z> + HasObservers<I, OT, S>,
    ST: Stats,
    OT: ObserversTuple<I, S> + serde::de::DeserializeOwned,
    Z: ExecutionProcessor<I, OT, S> + EvaluatorObservers<I, OT, S>,
{
}

impl<I, ST, OT, S> HasEventManagerId for MPIEventManager<I, ST, OT, S>
where
    I: Input,
    ST: Stats,
    OT: ObserversTuple<I, S> + serde::de::DeserializeOwned,
{
    fn mgr_id(&self) -> EventManagerId {
        EventManagerId { id: 0 }
    }
}

impl<I, ST, OT, S> MPIEventManager<I, ST, OT, S>
where
    I: Input,
    ST: Stats,
    OT: ObserversTuple<I, S> + serde::de::DeserializeOwned,
{
    /// Creates a new [`MPIEventManager`].
    pub fn new(stats: ST) -> Self {

        let (universe, _threading) = mpi::initialize_with_threading(Threading::Single).unwrap();

        let world_communicator = universe.world();
        let rank = world_communicator.rank();
        let size = world_communicator.size();

        Self {
            stats,
            communicator: world_communicator,
            processor_ct: size,
            rank,
            phantom: PhantomData{}
        }
    }

    // Handle arriving events in the broker
    #[allow(clippy::unnecessary_wraps)]
    fn handle_in_broker(stats: &mut ST, event: &Event<I>) -> Result<BrokerEventResult, Error> {
        match event {
            Event::NewTestcase {
                input: _,
                client_config: _,
                exit_kind: _,
                corpus_size,
                observers_buf: _,
                time,
                executions,
            } => {
                stats
                    .client_stats_mut_for(0)
                    .update_corpus_size(*corpus_size as u64);
                stats
                    .client_stats_mut_for(0)
                    .update_executions(*executions as u64, *time);
                stats.display(event.name().to_string(), 0);
                Ok(BrokerEventResult::Forward)
            }
            Event::UpdateStats {
                time,
                executions,
                phantom: _,
            } => {
                // TODO: The stats buffer should be added on client add.
                stats
                    .client_stats_mut_for(0)
                    .update_executions(*executions as u64, *time);
                stats.display(event.name().to_string(), 0);
                Ok(BrokerEventResult::Handled)
            }
            Event::UpdateUserStats {
                name,
                value,
                phantom: _,
            } => {
                stats
                    .client_stats_mut_for(0)
                    .update_user_stats(name.clone(), value.clone());
                stats.display(event.name().to_string(), 0);
                Ok(BrokerEventResult::Handled)
            }
            #[cfg(feature = "introspection")]
            Event::UpdatePerfStats {
                time,
                executions,
                introspection_stats,
                phantom: _,
            } => {
                // TODO: The stats buffer should be added on client add.
                stats.client_stats_mut()[0].update_executions(*executions as u64, *time);
                stats.client_stats_mut()[0]
                    .update_introspection_stats((**introspection_stats).clone());
                stats.display(event.name().to_string(), 0);
                Ok(BrokerEventResult::Handled)
            }
            Event::Objective { objective_size } => {
                stats
                    .client_stats_mut_for(0)
                    .update_objective_size(*objective_size as u64);
                stats.display(event.name().to_string(), 0);
                Ok(BrokerEventResult::Handled)
            }
            Event::Log {
                severity_level,
                message,
                phantom: _,
            } => {
                let (_, _) = (message, severity_level);
                #[cfg(feature = "std")]
                println!("[LOG {}]: {}", severity_level, message);
                Ok(BrokerEventResult::Handled)
            } //_ => Ok(BrokerEventResult::Forward),
        }
    }

    // Handle arriving events in the client
    #[allow(clippy::unused_self)]
    fn handle_in_client<E, Z>(
        &mut self,
        fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut S,
        _client_id: u32,
        event: Event<I>,
    ) -> Result<(), Error>
        where
            OT: ObserversTuple<I, S> + serde::de::DeserializeOwned,
            E: Executor<Self, I, S, Z> + HasObservers<I, OT, S>,
            Z: ExecutionProcessor<I, OT, S> + EvaluatorObservers<I, OT, S>,
    {
        match event {
            Event::NewTestcase {
                input,
                client_config,
                exit_kind,
                corpus_size: _,
                observers_buf,
                time: _,
                executions: _,
            } => {
                #[cfg(feature = "std")]
                println!(
                    "Received new Testcase from {} ({})",
                    _client_id, client_config
                );

                let observers: OT = postcard::from_bytes( & observers_buf)?;
                let _res = fuzzer.process_execution(state, self, input, &observers, &exit_kind, false)?;

                #[cfg(feature = "std")]
                if let Some(item) = _res.1 {
                    println!("Added received Testcase as item #{}", item);
                }
                Ok(())
            }
            _ => Err(Error::Unknown(format!(
                "Received illegal message that message should not have arrived: {:?}.",
                event.name()
            ))),
        }
    }
}