//! Coalescing background scene engraving for UI applications.
use crate::{engrave, RenderError, Scene, SceneOptions, Track};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;

struct SceneRequest {
    revision: u64,
    track: Track,
    options: SceneOptions,
}

/// Result produced by [`SceneWorker`]. Stale revisions can be discarded by the
/// application without comparing score models.
#[derive(Debug)]
pub struct AsyncSceneResult {
    /// Caller-provided model revision associated with the result.
    pub revision: u64,
    /// Completed scene or its validation/engraving error.
    pub scene: Result<Scene, RenderError>,
}

/// Single background engraving worker with request coalescing.
///
/// If edits arrive faster than engraving, queued requests are collapsed to the
/// newest revision before the next layout begins. Dropping the worker closes its
/// request channel and allows the thread to exit after its current layout.
pub struct SceneWorker {
    requests: Option<Sender<SceneRequest>>,
    results: Receiver<AsyncSceneResult>,
    handle: Option<JoinHandle<()>>,
}

impl SceneWorker {
    /// Starts the worker thread and creates its request and result channels.
    pub fn new() -> Result<Self, RenderError> {
        let (request_tx, request_rx) = mpsc::channel::<SceneRequest>();
        let (result_tx, result_rx) = mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("alphatab-scene".into())
            .spawn(move || {
                while let Ok(mut request) = request_rx.recv() {
                    for newer in request_rx.try_iter() {
                        if newer.revision >= request.revision {
                            request = newer;
                        }
                    }
                    let result = AsyncSceneResult {
                        revision: request.revision,
                        scene: engrave(&request.track, request.options),
                    };
                    if result_tx.send(result).is_err() {
                        break;
                    }
                }
            })
            .map_err(|error| {
                RenderError::worker(format!("could not start scene worker: {error}"))
            })?;
        Ok(Self {
            requests: Some(request_tx),
            results: result_rx,
            handle: Some(handle),
        })
    }

    /// Queues an owned track for engraving under the supplied revision number.
    pub fn request(
        &self,
        revision: u64,
        track: Track,
        options: SceneOptions,
    ) -> Result<(), RenderError> {
        self.requests
            .as_ref()
            .ok_or_else(|| RenderError::worker("scene worker has stopped"))?
            .send(SceneRequest {
                revision,
                track,
                options,
            })
            .map_err(|_| RenderError::worker("scene worker has stopped"))
    }

    /// Return the newest completed result currently available.
    pub fn try_take_latest(&self) -> Result<Option<AsyncSceneResult>, RenderError> {
        let mut latest: Option<AsyncSceneResult> = None;
        loop {
            match self.results.try_recv() {
                Ok(result)
                    if latest
                        .as_ref()
                        .is_none_or(|old| result.revision >= old.revision) =>
                {
                    latest = Some(result);
                }
                Ok(_) => {}
                Err(TryRecvError::Empty) => return Ok(latest),
                Err(TryRecvError::Disconnected) => {
                    return if latest.is_some() {
                        Ok(latest)
                    } else {
                        Err(RenderError::worker("scene worker has stopped"))
                    };
                }
            }
        }
    }

    /// Wait for one completed scene. Intended for non-UI callers and tests.
    pub fn receive_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<AsyncSceneResult, RenderError> {
        self.results.recv_timeout(timeout).map_err(|error| {
            RenderError::worker(format!("scene worker did not produce a result: {error}"))
        })
    }

    /// Stops accepting requests and waits for the current scene, if any, to
    /// finish. Consuming the worker prevents further requests after shutdown.
    pub fn shutdown(mut self) -> Result<(), RenderError> {
        self.stop_and_join()
    }

    fn stop_and_join(&mut self) -> Result<(), RenderError> {
        self.requests.take();
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| RenderError::worker("layout worker thread panicked"))?;
        }
        Ok(())
    }
}

impl Drop for SceneWorker {
    fn drop(&mut self) {
        // A dropped worker must not leave a background thread holding model
        // data alive. There is no useful way to report a panic from Drop.
        let _ = self.stop_and_join();
    }
}
