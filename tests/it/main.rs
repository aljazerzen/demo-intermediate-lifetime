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

use car::{Engine, Fuel};

struct FuelDep<'engine>(pub Fuel<'engine, 'engine>);

#[repr(transparent)]
struct EngineAndFuel<'car> {
    inner: std::pin::Pin<Box<pac_cell::PacInner<FuelDep<'static>, Engine<'car>>>>,
}
impl<'car> EngineAndFuel<'car> {
    fn new(
        parent: Engine<'car>,
        child_constructor: impl for<'a> ::core::ops::FnOnce(&'a mut Engine<'car>) -> FuelDep<'a>,
    ) -> Self {
        let inner = pac_cell::PacInner {
            parent,
            child: std::cell::OnceCell::new(),
            _pin: std::marker::PhantomPinned,
        };
        let mut inner = Box::pin(inner);
        let mut parent_ref = std::ptr::NonNull::from(&inner.as_mut().parent);
        let parent_ref: &mut Engine<'car> = unsafe { parent_ref.as_mut() };

        let child = child_constructor(parent_ref) as FuelDep<'static>;
        let _ = inner.child.set(child);

        EngineAndFuel { inner }
    }

    fn with_mut<R>(&mut self, f: impl FnOnce(&mut FuelDep<'_>) -> R) -> R {
        let mut_ref: std::pin::Pin<&mut pac_cell::PacInner<FuelDep, Engine<'car>>> =
            std::pin::Pin::as_mut(&mut self.inner);
        let inner = unsafe { std::pin::Pin::get_unchecked_mut(mut_ref) };
        let fuel = inner.child.get_mut().unwrap();
        f(fuel)
    }

    fn into_owned(self) -> Engine<'car> {
        let inner = unsafe { std::pin::Pin::into_inner_unchecked(self.inner) };
        inner.parent
    }
}

impl GetFluid for car::Car {
    type Item<'a> = EngineAndFuel<'a> where Self: 'a;

    fn get_fluid<'a>(&'a mut self) -> Self::Item<'a> {
        // create engine by borrowing self
        let engine: car::Engine<'a> = self.get_engine();

        EngineAndFuel::new(engine, init_fuel_dep)
    }
}

fn init_fuel_dep<'e, 'car: 'e>(e: &'e mut Engine<'car>) -> FuelDep<'e> {
    FuelDep(e.get_fuel())
}

#[test]
fn test_01() {
    let mut car = car::Car {
        engines: vec![3.2, 1.5],
    };

    {
        let mut fuel = car.get_fluid();

        fuel.with_mut(|f| f.0.update(4.2));
    }

    assert_eq!(car.engines, vec![4.2, 1.5]);
}

// #[test]
// fn test_02() {
//     let mut car = car::Car {
//         engines: vec![3.2, 1.5],
//     };

//     {
//         let mut fuel = car.get_fluid();

//         fuel.with_mut(|f| f.update(4.2));

//         let _engine = fuel.unwrap();
//     }

//     assert_eq!(car.engines, vec![4.2, 1.5]);
// }

// #[test]
// fn test_03() {
//     type Dep<'o> = &'o mut i64;

//     pac_cell::pac_cell!(
//         struct Hello {
//             owner: i64,
//             dependent: Dep,
//         }
//     );

//     let mut pac = Hello::new(10, |h| h);

//     let initial = pac.with_mut(|dep| {
//         let i = **dep;
//         **dep = 12;
//         i
//     });
//     assert_eq!(initial, 10);

//     let hello_again = pac.into_owned();
//     assert_eq!(hello_again, 12);
// }
