mod a1;
use a1::LockFreeQueue;
use a1::TwoLockQueue;

fn main() {
   let queue: LockFreeQueue<i32> = LockFreeQueue::new();
   queue.push(1);
   queue.push(2);
   queue.push(3);

   assert_eq!(queue.pop(), Some(1));
   assert_eq!(queue.pop(), Some(2));
   assert_eq!(queue.pop(), Some(3));
   assert_eq!(queue.pop(), None);
}
