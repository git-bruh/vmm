/// Wrap a value, executing the `cleanup` callback when it's dropped
pub struct WrappedAutoFree<T, F: FnMut(&T)> {
    pub val: T,
    cleanup: F,
}

impl<T, F: FnMut(&T)> WrappedAutoFree<T, F> {
    pub fn new(val: T, cleanup: F) -> Self {
        Self { val, cleanup }
    }
}

impl<T, F: FnMut(&T)> Drop for WrappedAutoFree<T, F> {
    fn drop(&mut self) {
        (self.cleanup)(&self.val);
    }
}
