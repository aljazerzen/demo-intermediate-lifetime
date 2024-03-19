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

use pac::Pac;

impl GetFluid for car::Car {
    type Item<'a> = Pac<car::Engine<'a>, car::Fuel<'a, 'a>> where Self: 'a;

    fn get_fluid<'a>(&'a mut self) -> Self::Item<'a> {
        // create engine by borrowing self
        let engine: car::Engine<'a> = self.get_engine();

        Pac::new(engine, |e| e.get_fuel())
    }
}

#[test]
fn test_01() {
    let mut car = car::Car {
        engines: vec![3.2, 1.5],
    };

    {
        let mut fuel = car.get_fluid();

        fuel.with_mut(|f| f.update(4.2));
    }

    assert_eq!(car.engines, vec![4.2, 1.5]);
}

#[test]
fn test_02() {
    let mut car = car::Car {
        engines: vec![3.2, 1.5],
    };

    {
        let mut fuel = car.get_fluid();

        fuel.with_mut(|f| f.update(4.2));

        let _engine = fuel.unwrap();
    }

    assert_eq!(car.engines, vec![4.2, 1.5]);
}

#[test]
fn test_03() {
    struct Hello {
        world: i64,
    }
    let hello = Hello { world: 10 };

    let mut pac = Pac::new(hello, |h| &mut h.world);

    let initial = pac.with_mut(|world| {
        let i = **world;
        **world = 12;
        i
    });
    assert_eq!(initial, 10);

    let hello_again = pac.unwrap();
    assert_eq!(hello_again.world, 12);
}
