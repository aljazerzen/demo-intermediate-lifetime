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
        id: usize,
        pub car: &'car mut Car,
        // + some internal fields
    }

    pub struct Fuel<'engine> {
        pub engine: &'engine usize,
        // + some internal fields
    }

    impl Car {
        pub fn get_engine(&mut self) -> Engine<'_> {
            println!("create engine");
            Engine { id: 0, car: self }
        }
    }

    impl<'car> Engine<'car> {
        pub fn get_fuel(&self) -> Fuel<'_> {
            println!("create fuel");
            Fuel { engine: &self.id }
        }
    }

    impl<'a> Drop for Engine<'a> {
        fn drop(&mut self) {
            println!("drop engine");
        }
    }

    impl<'a> Drop for Fuel<'a> {
        fn drop(&mut self) {
            println!("drop fuel");
        }
    }
}

use std::marker::PhantomPinned;
use std::ptr::NonNull;
use std::{cell::OnceCell, pin::Pin};

use car::{Engine, Fuel};

pub struct EngineAndFuel<'car> {
    engine: Engine<'car>,
    engine_ref: NonNull<Engine<'car>>,
    fuel: OnceCell<Fuel<'car>>,
    _pin: PhantomPinned,
}

impl GetFluid for car::Car {
    type Item<'a> = Pin<Box<EngineAndFuel<'a>>> where Self: 'a;

    fn get_fluid<'a>(&'a mut self) -> Self::Item<'a> {
        let engine = self.get_engine();

        // this here is the main problem:
        // I cannot express lifetime of this `engine`, since it is a local variable.
        // What I want, is for it to exist as long as the returned values exists.
        // ... so I put it into a combined struct together with fuel.
        // But this is now a self-referential struct, so I must use a bit of Pin magic.

        let res = EngineAndFuel {
            engine,
            engine_ref: NonNull::dangling(),
            fuel: OnceCell::new(),
            _pin: PhantomPinned,
        };
        let mut boxed = Box::pin(res);

        let engine_ref = NonNull::from(&boxed.engine);
        unsafe {
            let mut_ref: Pin<&mut EngineAndFuel> = Pin::as_mut(&mut boxed);
            Pin::get_unchecked_mut(mut_ref).engine_ref = engine_ref;
        }

        let fuel = unsafe { boxed.engine_ref.as_ref().get_fuel() };
        let _ = boxed.fuel.set(fuel);

        boxed
    }
}

impl<'car> EngineAndFuel<'car> {
    fn update(&mut self, val: f64) {
        let engine_id = *self.fuel.get().unwrap().engine;
        self.engine.car.engines[engine_id] = val;
    }
}

fn main() {
    let mut car = car::Car {
        engines: vec![3.2, 1.5],
    };

    {
        let mut fuel = car.get_fluid();

        // even using the returned type in inconvenient...
        unsafe {
            let mut_ref: Pin<&mut EngineAndFuel> = Pin::as_mut(&mut fuel);
            Pin::get_unchecked_mut(mut_ref).update(2.3);
        }
    }

    println!("{:?}", car.engines);
}
