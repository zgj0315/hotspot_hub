use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct RingBuffer<T> {
    capacity: usize,
    values: VecDeque<T>,
}

impl<T: Clone> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            values: VecDeque::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, value: T) {
        if self.capacity == 0 {
            return;
        }
        while self.values.len() >= self.capacity {
            self.values.pop_front();
        }
        self.values.push_back(value);
    }

    pub fn to_vec(&self) -> Vec<T> {
        self.values.iter().cloned().collect()
    }
}
