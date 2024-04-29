use std::sync::{Arc, Mutex};

pub struct DeadDrop<T> {
    pub value: Arc<Mutex<Option<T>>>,
}

impl<T> Clone for DeadDrop<T> {
    fn clone(&self) -> Self {
        DeadDrop {
            value: self.value.clone(),
        }
    }
}

impl<T> Default for DeadDrop<T> {
    fn default() -> Self {
        DeadDrop {
            value: Arc::new(Mutex::new(None)),
        }
    }
}

impl<T> DeadDrop<T> {
    pub fn send(&self, new_value: T) {
        let mut v = self.value.lock().unwrap();
        *v = Some(new_value);
    }

    pub fn recv(&self) -> Option<T> {
        let mut v = self.value.lock().unwrap();
        std::mem::replace(&mut *v, None)
    }
}
