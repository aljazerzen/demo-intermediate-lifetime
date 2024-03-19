/// The trait I want to implement, but cannot change.
pub trait GetFluid {
    type Item<'a>
    where
        Self: 'a;

    fn get_fluid<'a>(&'a mut self) -> Self::Item<'a>;
}

/// A library I want to use, but it has a bit different api:
/// the difference is that Car does not yield Fuel directly, but via Engine.
mod car {
    pub struct Car {
        pub engines: Vec<f64>,
    }

    pub struct Engine<'car> {
        pub id: usize,
        pub car: &'car mut Car,
        // + some internal fields
    }

    pub struct Fuel<'car, 'engine> {
        pub engine: &'engine mut Engine<'car>,
        // + some internal fields
    }

    impl Car {
        pub fn get_engine(&mut self) -> Engine<'_> {
            println!("create engine");
            Engine { id: 0, car: self }
        }
    }

    impl<'car> Engine<'car> {
        pub fn get_fuel(&mut self) -> Fuel<'car, '_> {
            println!("create fuel");
            Fuel { engine: self }
        }
    }

    impl<'car, 'engine> Fuel<'car, 'engine> {
        pub fn update(&mut self, val: f64) {
            self.engine.car.engines[self.engine.id] = val;
        }
    }

    impl<'a> Drop for Engine<'a> {
        fn drop(&mut self) {
            println!("drop engine");
        }
    }

    impl<'a, 'b> Drop for Fuel<'a, 'b> {
        fn drop(&mut self) {
            println!("drop fuel");
        }
    }
}

use std::marker::PhantomPinned;
use std::ptr::NonNull;
use std::{cell::OnceCell, pin::Pin};

use car::{Engine, Fuel};

/// Inner object of [Pac].
///
/// ## Safety
///
/// While this struct exist, the parent is considered mutably borrowed.
/// Therefore, any access to parent is UB.
///
/// Because child might contain pointers to parent, this struct cannot
/// be moved.
pub struct PacInner<'car> {
    child: OnceCell<Fuel<'car, 'car>>,
    parent: Engine<'car>,
    _pin: PhantomPinned,
}

/// Parent and a child that is created by mutably borrowing the parent.
/// Allows mutable access to the child.
pub struct Pac<'car>(Pin<Box<PacInner<'car>>>);

impl<'car> Pac<'car> {
    fn new<F>(parent: Engine<'car>, child_constructor: F) -> Self
    where
        F: for<'e> FnOnce(&'e mut Engine<'car>) -> Fuel<'car, 'e>,
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
        let child = child_constructor(parent_ref);
        let _ = inner.child.set(child);

        Pac(inner)
    }

    pub fn with_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Fuel<'car, 'car>) -> R,
    {
        let mut_ref: Pin<&mut PacInner> = Pin::as_mut(&mut self.0);

        // SAFETY: this is safe because we don't move the inner pinned object
        let inner = unsafe { Pin::get_unchecked_mut(mut_ref) };
        let fuel = inner.child.get_mut().unwrap();

        f(fuel)
    }

    pub fn unwrap(self) -> Engine<'car> {
        // SAFETY: this is safe because child is dropped when this function finishes,
        //    but parent still exists.
        let inner = unsafe { Pin::into_inner_unchecked(self.0) };
        inner.parent
    }
}

impl GetFluid for car::Car {
    type Item<'a> = Pac<'a> where Self: 'a;

    fn get_fluid<'a>(&'a mut self) -> Self::Item<'a> {
        // create engine by borrowing self
        let engine: Engine<'a> = self.get_engine();

        Pac::new(engine, |e| e.get_fuel())
    }
}

fn main() {
    let mut car = car::Car {
        engines: vec![3.2, 1.5],
    };

    {
        println!("get_fluid");
        let mut fuel = car.get_fluid();
        
        println!("with_mut");
        fuel.with_mut(|f| f.update(4.2));

        println!("unwrap");
        let _engine = fuel.unwrap();
        println!("_engine");
    }

    println!("{:?}", car.engines);
}
