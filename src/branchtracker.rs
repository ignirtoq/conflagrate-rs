use std::sync::atomic::{AtomicBool, AtomicI32};
use std::sync::atomic::Ordering::Relaxed;
use tokio::sync::oneshot;
use tokio::sync::oneshot::{Receiver, Sender};

pub struct BranchTracker<T> {
    num_branches: AtomicI32,
    sender: Option<Sender<T>>,
    done: AtomicBool,
}
impl<T> BranchTracker<T> {
    pub fn new() -> (Receiver<T>, BranchTracker<T>) {
        let (sender, receiver) = oneshot::channel();
        (receiver, BranchTracker{
            num_branches: AtomicI32::new(1),
            sender: Some(sender),
            done: AtomicBool::new(false)
        })
    }

    pub fn add_branch(&self) {
        if self.done.load(Relaxed) { return; }
        self.num_branches.fetch_add(1, Relaxed);
    }

    pub fn remove_branch(&mut self, last_node_output: T) {
        if self.done.load(Relaxed) { return; }
        self.num_branches.fetch_add(-1, Relaxed);
        if self.num_branches.load(Relaxed) <= 0 {
            self.done.store(true, Relaxed);
            if let Some(sender) = self.sender.take() {
                match sender.send(last_node_output) {
                    _ => {}
                }
            }
        }
    }
}
