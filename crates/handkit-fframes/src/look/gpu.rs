//! GPU renders, one at a time on a thread of their own.
//!
//! fframes_three_d's GPU renderer keeps three frame contexts and makes a render wait for a
//! free one, and parts of a render (the environment map, the post-process) run rayon tasks
//! while it holds a context or a lock. Called from rayon workers (fframes renders frames on
//! them, and a look renders its passes with `rayon::join`), a worker blocked in such a task
//! steals another frame's render, which then waits for a context or lock the first one holds:
//! every thread waits and the video stops. Here every GPU render runs on one thread outside
//! rayon's pool, with a pool of its own for the render's parallel parts, and the caller waits
//! for the pixels.

use std::sync::mpsc::{Sender, channel};
use std::sync::{Mutex, OnceLock};

use fframes_three_d::{Camera, Pixels, Renderer, Scene};

type Job = Box<dyn FnOnce() + Send>;

fn worker() -> &'static Mutex<Sender<Job>> {
    static WORKER: OnceLock<Mutex<Sender<Job>>> = OnceLock::new();
    WORKER.get_or_init(|| {
        let (send, jobs) = channel::<Job>();
        std::thread::Builder::new()
            .name("handkit-gpu".into())
            .spawn(move || {
                let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
                let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).thread_name(|i| format!("handkit-gpu-{i}")).build().expect("a thread pool");
                for job in jobs {
                    pool.install(job);
                }
            })
            .expect("a GPU thread");
        Mutex::new(send)
    })
}

/// `renderer.render_pixels(scene, camera)`: on the GPU thread when the renderer renders on
/// the GPU, here otherwise.
pub(crate) fn render_pixels(renderer: &Renderer, scene: Scene, camera: &Camera) -> Pixels {
    if !renderer.uses_gpu() {
        return renderer.render_pixels(&scene, camera);
    }
    let (renderer, camera) = (renderer.clone(), *camera);
    let (send, reply) = channel();
    let job: Job = Box::new(move || {
        let _ = send.send(renderer.render_pixels(&scene, &camera));
    });
    worker().lock().unwrap_or_else(|e| e.into_inner()).send(job).expect("the GPU thread is running");
    reply.recv().expect("the GPU thread renders every job")
}
