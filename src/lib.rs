//! Provides [PacCell] (a cell of a parent and a child).

use std::marker::PhantomPinned;
use std::ptr::NonNull;
use std::{cell::OnceCell, pin::Pin};

/// A cell of a parent and a child, which is created by mutably borrowing the parent.
/// While the parent is in the cell, it cannot be accessed in any way.
/// Provides mutable access to the child.
///
/// This is useful in a rare case when you need to store and move both
/// parent and their child together.
///
/// ## Examples
///
/// Basic usage:
/// ```
/// struct Hello {
///     world: i64,
/// }
/// let hello = Hello { world: 10 };
///
/// let mut pac = pac_cell::PacCell::new(hello, |h| &mut h.world);
///
/// let initial = pac.with_mut(|world| {
///     let i = **world;
///     **world = 12;
///     i
/// });
/// assert_eq!(initial, 10);
///
/// let hello_again = pac.unwrap();
/// assert_eq!(hello_again.world, 12);
/// ```
///
/// For a real-world-like example, see the crate tests.
pub struct PacCell<P, C>(Pin<Box<PacInner<P, C>>>);

/// Inner object of [Pac].
///
/// ## Safety
///
/// While this struct exist, the parent is considered mutably borrowed.
/// Therefore, any access to parent is UB.
///
/// Because child might contain pointers to parent, this struct cannot
/// be moved.
struct PacInner<P, C> {
    /// Child has to be defined before the parent, so it is dropped
    /// before the parent
    child: OnceCell<C>,
    parent: P,

    /// Mark this struct as non-movable. Not really needed, since we always
    /// have it in `Pin<Box<_>>``, but there is no hard in being too explicit.
    _pin: PhantomPinned,
}

impl<'p, P: 'p, C> PacCell<P, C> {
    /// Creates Pac by moving the parent into a [Box] and then calling
    /// the child constructor.
    pub fn new<F>(parent: P, child_constructor: F) -> Self
    where
        F: FnOnce(&'p mut P) -> C,
    {
        Self::try_new::<_, ()>(parent, |p| Ok(child_constructor(p))).unwrap()
    }

    /// Creates Pac by moving the parent into a [Box] and then calling
    /// the child constructor.
    pub fn try_new<F, E>(parent: P, child_constructor: F) -> Result<Self, E>
    where
        F: FnOnce(&'p mut P) -> Result<C, E>,
    {
        // move engine into the struct and pin the struct on heap
        let inner = PacInner {
            parent,
            child: OnceCell::new(),
            _pin: PhantomPinned,
        };
        let mut inner = Box::pin(inner);

        // create mut reference to engine, without borrowing the struct
        // SAFETY: generally this would be unsafe, since one could obtain multiple mut refs this way.
        //   But because we don't allow any access to engine, this mut reference is guaranteed
        //   to be the only one.
        let mut parent_ref = NonNull::from(&inner.as_mut().parent);
        let parent_ref = unsafe { parent_ref.as_mut() };

        // create fuel and move it into the struct
        let child = child_constructor(parent_ref)?;
        let _ = inner.child.set(child);

        Ok(PacCell(inner))
    }

    /// Executes a function with a mutable reference to the child.
    pub fn with_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut C) -> R,
    {
        let mut_ref: Pin<&mut PacInner<P, C>> = Pin::as_mut(&mut self.0);

        // SAFETY: this is safe because we don't move the inner pinned object
        let inner = unsafe { Pin::get_unchecked_mut(mut_ref) };
        let fuel = inner.child.get_mut().unwrap();

        f(fuel)
    }

    /// Drop the child and return the parent.
    pub fn unwrap(self) -> P {
        // SAFETY: this is safe because child is dropped when this function finishes,
        //    but parent still exists.
        let inner = unsafe { Pin::into_inner_unchecked(self.0) };
        inner.parent
    }
}
