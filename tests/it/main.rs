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

use car::{Fuel, Engine};

struct FuelDep<'engine>(pub Fuel<'engine, 'engine>);

pac_cell::pac_cell!(
    struct EngineAndFuel<'car> {
        owner: Engine<'car>,

        dependent: FuelDep,
    }
);

impl GetFluid for car::Car {
    type Item<'a> = EngineAndFuel<'a> where Self: 'a;

    fn get_fluid<'a>(&'a mut self) -> Self::Item<'a> {
        // create engine by borrowing self
        let engine: car::Engine<'a> = self.get_engine();

        EngineAndFuel::new(engine, |e| FuelDep(e.get_fuel()))
    }
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

#[test]
fn test_03() {
    type Dep<'o> = &'o mut i64;

    pac_cell::pac_cell!(
        struct Hello {
            owner: i64,
            dependent: Dep,
        }
    );

    let mut pac = Hello::new(10, |h| h);

    let initial = pac.with_mut(|dep| {
        let i = **dep;
        **dep = 12;
        i
    });
    assert_eq!(initial, 10);

    let hello_again = pac.into_owned();
    assert_eq!(hello_again, 12);
}
