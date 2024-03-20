//! Provides [PacCell] (a cell of a parent and a child).

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
/// let mut pac = pac::Pac::new(hello, |h| &mut h.world);
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
#[macro_export]
macro_rules! pac_cell {
(
    $(#[$StructMeta:meta])*
    $Vis:vis struct $StructName:ident $(<$OwnerLifetime:lifetime>)? {
        owner: $Owner:ty,

        dependent: $Dependent:ident,
    }

    $(impl {$($AutomaticDerive:ident),*})?
) => {
    #[repr(transparent)]
    $(#[$StructMeta])*
    $Vis struct $StructName $(<$OwnerLifetime>)? {
        inner: std::pin::Pin<Box<$crate::PacInner<$Owner, $Dependent<'static>>>>
    }

    impl $(<$OwnerLifetime>)? $StructName $(<$OwnerLifetime>)? {

        /// Creates Pac by moving the owner into a [Box] and then calling
        /// the dependent constructor.
        $Vis fn new(
            parent: $Owner,
            child_constructor: impl for<'a> FnOnce(&'a mut $Owner) -> $Dependent<'a>
        ) -> Self {
            Self::try_new::<()>(parent, |p| Ok(child_constructor(p))).unwrap()
        }

        /// Creates Pac by moving the parent into a [Box] and then calling
        /// the child constructor.
        $Vis fn try_new<E>(
            parent: $Owner,
            child_constructor: impl for<'a> FnOnce(&'a mut $Owner) -> Result<$Dependent<'a>, E>
        ) -> Result<Self, E> {
            // move engine into the struct and pin the struct on heap
            let inner = $crate::PacInner {
                parent,
                child: std::cell::OnceCell::new(),
                _pin: std::marker::PhantomPinned,
            };
            let mut inner = Box::pin(inner);

            // create mut reference to engine, without borrowing the struct
            // SAFETY: generally this would be unsafe, since one could obtain multiple mut refs this way.
            //   But because we don't allow any access to engine, this mut reference is guaranteed
            //   to be the only one.
            let mut parent_ref = std::ptr::NonNull::from(&inner.as_mut().parent);
            let parent_ref = unsafe { parent_ref.as_mut() };

            // create fuel and move it into the struct
            let child = child_constructor(parent_ref)?;
            let _ = inner.child.set(child);

            Ok($StructName { inner })
        }

        /// Executes a function with a mutable reference to the child.
        $Vis fn with_mut<R>(
            &mut self,
            f: impl FnOnce(&mut $Dependent<'_>) -> R
        ) -> R {
            let mut_ref: std::pin::Pin<&mut $crate::PacInner<$Owner, $Dependent>> =
                std::pin::Pin::as_mut(&mut self.inner);

            // SAFETY: this is safe because we don't move the inner pinned struct
            let inner = unsafe { std::pin::Pin::get_unchecked_mut(mut_ref) };
            let fuel = inner.child.get_mut().unwrap();

            f(fuel)
        }

        /// Drop the dependent and return the owned.
        $Vis fn into_owned(self) -> $Owner {
            // SAFETY: this is safe because owned is dropped when this function finishes,
            //    but dependent still exists.
            let inner = unsafe { std::pin::Pin::into_inner_unchecked(self.inner) };
            inner.parent
        }
    }

    // The user has to choose which traits can and should be automatically
    // implemented for the cell.
    $($(
        $crate::_impl_automatic_derive!($AutomaticDerive, $StructName);
    )*)*
};
}

/// Inner object of [Pac].
///
/// ## Safety
///
/// While this struct exist, the parent is considered mutably borrowed.
/// Therefore, any access to parent is UB.
///
/// Because child might contain pointers to parent, this struct cannot
/// be moved.
#[doc(hidden)]
pub struct PacInner<P, C> {
    /// Child has to be defined before the parent, so it is dropped
    /// before the parent
    pub child: std::cell::OnceCell<C>,
    pub parent: P,

    /// Mark this struct as non-movable. Not really needed, since we always
    /// have it in `Pin<Box<_>>``, but there is no hard in being too explicit.
    pub _pin: std::marker::PhantomPinned,
}
