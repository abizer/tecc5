use std::sync::{Arc, Mutex};
use std::cell::UnsafeCell;
use std::ptr;

struct Node<T> {
    data: T,
    next: *mut Node<T>,
}

pub struct TwoLockQueue<T> {
    head: Mutex<*mut Node<T>>,
    tail: Mutex<*mut Node<T>>,
    dummy: UnsafeCell<*mut Node<T>>, // to safely share mutable data across threads
}

unsafe impl<T> Sync for TwoLockQueue<T> {}
unsafe impl<T> Send for TwoLockQueue<T> {}

impl<T> TwoLockQueue<T> {
    pub fn new() -> Arc<TwoLockQueue<T>> {
        let dummy = Box::new(Node {
            data: unsafe { std::mem::zeroed() }, // Use a placeholder   
            next: ptr::null_mut(),
        });
        let dummy_ptr = Box::into_raw(dummy);

        Arc::new(TwoLockQueue {
            head: Mutex::new(dummy_ptr),
            tail: Mutex::new(dummy_ptr),
            dummy: UnsafeCell::new(dummy_ptr),
        })
    }

    pub fn push(&self, data: T) {
        // create the new node
        let new_node = Box::new(Node {
            data,
            next: ptr::null_mut(),
        });
        let new_node_ptr = Box::into_raw(new_node);

        // lock the tailptr and set its nextptr to the new node we just created
        let mut tailx = self.tail.lock().unwrap();
        unsafe {
            // first deref whatever tailx is pointing to, then deref its nextptr
            // and set it, adding our new node to the queue
            (*(*tailx)).next = new_node_ptr;
        }
        // advance the tailptr to the new node. the mutex gets released when this goes
        // out of scope
        *tailx = new_node_ptr;
    }

    pub fn pop(&self) -> Option<T> {
        // lock the headptr
        let mut head = self.head.lock().unwrap();
        // so we can get its nextptr (the node we want to pop)
        let next = unsafe { (*(*head)).next };
        // if the nextptr is not null, we have a node to pop
        if !next.is_null() {
            // deref the nextptr to get the node + data
            let next_node = unsafe { &*next };
            let data = unsafe { ptr::read(&next_node.data) };
            // advance the headptr
            *head = next;
            return Some(data);
        } else {
            // if the nextptr is null, we have nothing to pop
            None
        }
    }
}

impl<T> Drop for TwoLockQueue<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
        unsafe { drop(Box::from_raw(*self.dummy.get())); } // Free the last dummy node
    }
}
