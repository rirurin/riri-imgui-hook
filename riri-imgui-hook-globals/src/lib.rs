#![allow(dead_code)]

pub struct UnsafePtr<T>(*mut T);
unsafe impl<T> Send for UnsafePtr<T> {}
unsafe impl<T> Sync for UnsafePtr<T> {}

impl<T> std::fmt::Debug for UnsafePtr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TODO!")
    }
}

pub mod globals;