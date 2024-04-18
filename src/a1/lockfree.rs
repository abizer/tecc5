use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

struct Node<T> {
    data: T,
    next: AtomicPtr<Node<T>>,
}

impl<T> Node<T> {
    fn new(data: T) -> Node<T> {
        Node {
            data,
            next: AtomicPtr::new(ptr::null_mut()),
        }
    }
}

pub struct LockFreeQueue<T> {
    head: AtomicPtr<Node<T>>,
    tail: AtomicPtr<Node<T>>,
}

impl<T> LockFreeQueue<T> {
    pub fn new() -> Self {
        let dummy = Box::new(Node::new(unsafe { std::mem::zeroed() }));
        let dummy_ptr = Box::into_raw(dummy);
        LockFreeQueue {
            head: AtomicPtr::new(dummy_ptr),
            tail: AtomicPtr::new(dummy_ptr),
        }
    }

    pub fn push(&self, data: T) {
        // create a new node
        let new_node = Box::new(Node::new(data));
        let new_node_ptr = Box::into_raw(new_node);

        loop {
            // acquire the tailptr to prepare for the push
            let tail_ptr = self.tail.load(Ordering::Acquire);
            let tail_next_ptr = unsafe { (*tail_ptr).next.load(Ordering::Acquire) };

            // try to acquire the tailptr. if it fails, loop
            if tail_ptr == self.tail.load(Ordering::Acquire) {
                // if the tailptr is the last node, (bc the nextptr is null)
                // try to cmpxchg the nextptr to the new node
                if tail_next_ptr.is_null() {
                    if unsafe {
                        (*tail_ptr)
                            .next
                            .compare_exchange(
                                ptr::null_mut(),
                                new_node_ptr,
                                Ordering::Release,
                                Ordering::Relaxed,
                            )
                            .is_ok()
                    } {
                        // we succeeded. cmpxchg to advance tailptr to the new node.
                        // if this fails, it's ok: tailptr.next already points to the new node,
                        // so we don't lose data. next time this loops, the previous if will fail,
                        // and the else case will advance the tailptr because tail_next_ptr will be
                        // pointing to new_node_ptr on the next iteration. so we can break the loop regardless
                        self.tail
                            .compare_exchange(
                                tail_ptr,
                                new_node_ptr,
                                Ordering::Release,
                                Ordering::Relaxed,
                            )
                            .ok();
                        break;
                    }
                } else {
                    // cleanup: try to advance tailptr to the next node, to maintain the invariant that tail.next is null
                    self.tail
                        .compare_exchange(
                            tail_ptr,
                            tail_next_ptr,
                            Ordering::Release,
                            Ordering::Relaxed,
                        )
                        .ok();
                }
            }
        }
    }

    pub fn pop(&self) -> Option<T> {
        loop {
            // acquire the head/tailptr to prepare for the loop
            let head_ptr = self.head.load(Ordering::Acquire);
            let tail_ptr = self.tail.load(Ordering::Acquire);
            let head_next_ptr = unsafe { (*head_ptr).next.load(Ordering::Acquire) };

            // if we got it, continue, else loop
            if head_ptr == self.head.load(Ordering::Acquire) {
                // head and tail point to the same thing, which means they both point to the dummy node
                if head_ptr == tail_ptr {
                    // queue is empty. return None
                    if head_next_ptr.is_null() {
                        return None;
                    }
                    // head_next_ptr isn't null, which means there an item in the queue. advance
                    // tailptr to the next node, bc it should point to the end of the queue. if this
                    // fails, it will be caught on the next iter.
                    self.tail
                        .compare_exchange(
                            tail_ptr,
                            head_next_ptr,
                            Ordering::Release,
                            Ordering::Relaxed,
                        )
                        .ok();
                } else {
                    // head and tail point to different things, which means there's data in the queue.
                    let res = unsafe { ptr::read(&(*head_next_ptr).data) };
                    // we read the data so cmpxchg to advance the headptr to its next node. we know its valid bc
                    // at worst it could be the tailptr, if it weren't, the previous if would have caught it.
                    // if it fails, loop and try again.
                    if self
                        .head
                        .compare_exchange(
                            head_ptr,
                            head_next_ptr,
                            Ordering::Release,
                            Ordering::Relaxed,
                        )
                        .is_ok()
                    {
                        // make a Box out of it so that when this goes out of scope the ptr gets freed
                        unsafe {
                            drop(Box::from_raw(head_ptr));
                        }
                        return Some(res);
                    }
                }
            }
        }
    }
}

impl<T> Drop for LockFreeQueue<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
        // loop over everything and Box it to free the mem when it goes out of scope
        unsafe {
            drop(Box::from_raw(self.head.load(Ordering::SeqCst)));
        }
    }
}
