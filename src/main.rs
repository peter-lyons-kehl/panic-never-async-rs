use core::future::Future;
use core::pin::Pin;
//use std::sync::mpsc::{channel, Receiver, Sender};
//use std::sync::{Arc, Mutex};
use core::task::{Context, RawWaker, RawWakerVTable, Waker};

/// A simple task container that holds a pinned future
struct Task {
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
    sender: Sender<Arc<Task>>,
}

/// The spawner that pushes new tasks into the queue
#[derive(Clone)]
struct Spawner {
    sender: Sender<Arc<Task>>,
}

impl Spawner {
    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
            sender: self.sender.clone(),
        });
        let _ = self.sender.send(task);
    }
}

/// The executor that pulls and polls tasks until the queue is empty
struct Executor {
    receiver: Receiver<Arc<Task>>,
}

impl Executor {
    fn run(&self) {
        while let Ok(task) = self.receiver.recv() {
            let waker = waker_ref(&task);
            let mut cx = Context::from_waker(&waker);
            let mut future = task.future.lock().unwrap();

            // Poll the future; if it's not ready, the waker will re-enqueue the task when woken
            let _ = future.as_mut().poll(&mut cx);
        }
    }
}

/// Helper function to create a `Waker` from an `Arc<Task>`
fn waker_ref(task: &Arc<Task>) -> Waker {
    // Clone the Arc and turn it into a raw pointer
    let ptr = Arc::into_raw(task.clone()) as *const ();

    // Define the vtable for raw waker manipulation
    unsafe fn clone(ptr: *const ()) -> RawWaker {
        Arc::increment_strong_count(ptr as *const Task);
        RawWaker::new(ptr, &VTABLE)
    }

    unsafe fn wake(ptr: *const ()) {
        let task = Arc::from_raw(ptr as *const Task);
        let _ = task.sender.send(task.clone());
    }

    unsafe fn wake_by_ref(ptr: *const ()) {
        let task = Arc::from_raw(ptr as *const Task);
        let _ = task.sender.send(task.clone());
        // Prevent `Arc::drop` from prematurely decrementing count since we didn't consume ownership here
        std::mem::forget(task);
    }

    unsafe fn drop_waker(ptr: *const ()) {
        let _ = Arc::from_raw(ptr as *const Task);
    }

    const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop_waker);

    unsafe { Waker::from_raw(RawWaker::new(ptr, &VTABLE)) }
}

/// Entry point to initialize and run the dummy runtime
fn new_runtime_and_spawn<F>(fut: F)
where
    F: Future<Output = ()> /**/ + Send /**/ + 'static,
{
    let (sender, receiver) = channel();
    let spawner = Spawner { sender };
    let executor = Executor { receiver };

    spawner.spawn(fut);
    executor.run();
}

fn main() {
    let future = async {
        println!("Hello from async task!");
    };
    new_runtime_and_spawn(future);
}
