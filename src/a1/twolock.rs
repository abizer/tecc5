use std::ptr; 
use std::sync::{Mutex, Arc};

struct Node<T> {
    data: T,
    next: Option<Arc<Mutex<Node<T>>>>,
}

