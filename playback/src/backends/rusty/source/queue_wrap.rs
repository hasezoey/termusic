use std::{sync::Arc, time::Duration};

use rodio::{queue::SourcesQueueOutput, source::SeekError, Source};

use super::SampleType;

// This type is similar to `rodio::source::Done`, but that type has one function, but we actually accept a function

/// Call a function at the end of the inner source.
pub struct QueueOutputWrap {
    queue: SourcesQueueOutput,
    pub test: Arc<()>
}

impl QueueOutputWrap
{
    /// Wrap the `input` source in a Done Callback that calls a function.
    #[inline]
    pub fn new(queue_rx: SourcesQueueOutput) -> Self {
        QueueOutputWrap {
            queue: queue_rx,
            test: Arc::new(()),
        }
    }
}

impl Iterator for QueueOutputWrap
{
    type Item = SampleType;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.queue.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.queue.size_hint()
    }
}

impl Source for QueueOutputWrap
{
    #[inline]
    fn current_span_len(&self) -> Option<usize> {
        self.queue.current_span_len()
    }

    fn channels(&self) -> u16 {
        self.queue.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.queue.sample_rate()
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.queue.total_duration()
    }

    #[inline]
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.queue.try_seek(pos)
    }
}

impl Drop for QueueOutputWrap {
    fn drop(&mut self) {
        error!("Queue RX Exit");
    }
}
