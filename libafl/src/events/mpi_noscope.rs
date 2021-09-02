use mpi::request::Scope;

/// A temporary scope that lasts no more than the lifetime `'a`
///
/// Use `NoScope` for to perform requests with temporary buffers.
///
/// To obtain a `NoScope`, use the [`no_scope`](fn.no_scope.html) function.
///
/// # Invariant
///
/// For any `Request` registered with a `NoScope<'a>`, its associated buffers must outlive `'a`.
#[derive(Debug)]
pub struct NoScope {
}

impl Drop for NoScope {
    fn drop(&mut self) {
        // Do nothing! Don't babysit me
    }
}

unsafe impl<'a> Scope<'a> for NoScope {
    fn register(&self) {}

    unsafe fn unregister(&self) {}
}