use mpi::request::Scope;
use core::marker::PhantomData;

/// A temporary scope that lasts no more than the lifetime `'a`
///
/// Use `NoScope` for to perform requests with temporary buffers.
///
/// To obtain a `NoScope`, use the [`no_scope`](fn.no_scope.html) function.
///
/// # Invariant
///
/// For any `Request` registered with a `NoScope<'a>`, its associated buffers must outlive `'a`.
#[derive(Debug, Clone, Copy)]
pub struct NoScope<'a> {
    phantom: PhantomData<&'a ()>
}

unsafe impl<'a> Scope<'a> for NoScope<'a> {
    fn register(&self) {}

    unsafe fn unregister(&self) {}
}

pub fn get_scope<'a>() -> NoScope<'a> {
    NoScope{
        phantom: PhantomData{}
    }
}