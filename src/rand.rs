use std;
use stdweb::unstable::TryInto;


const PERCENTAGE_MAX: f64 = 1.0 + std::f64::EPSILON;


fn rand() -> f64 {
    js!( return Math.random(); ).try_into().unwrap()
}

// TODO verify that this is correct
pub fn bool() -> bool {
    rand() < 0.5
}

// TODO verify that this is correct
pub fn percentage() -> f64 {
    rand() * PERCENTAGE_MAX
}

// TODO verify that this is correct
pub fn between_exclusive(min: u32, max: u32) -> u32 {
    let range = (max - min) as f64;
    let x = (rand() * range).floor() as u32;
    x + min
}

// TODO verify that this is correct
pub fn between_inclusive(min: u32, max: u32) -> u32 {
    let range = ((max - min) + 1) as f64;
    let x = (rand() * range).floor() as u32;
    x + min
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::{Debug, Error, Formatter};
    use std::cmp::Ordering;
    use std::collections::BTreeMap;

    #[derive(PartialOrd, PartialEq)]
    struct OrdWrap<A>(A);

    impl<A: Debug> Debug for OrdWrap<A> {
        fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
            self.0.fmt(f)
        }
    }

    impl<A: PartialOrd> Ord for OrdWrap<A> {
        fn cmp(&self, other: &Self) -> Ordering {
            self.partial_cmp(other).unwrap()
        }
    }

    impl<A: PartialEq> Eq for OrdWrap<A> {}

    fn test_distribution<A: Debug + PartialOrd + PartialEq, F: FnMut() -> A>(name: &str, mut f: F) {
        let mut counts = BTreeMap::new();

        for _ in 0..1000000 {
            *counts.entry(OrdWrap(f())).or_insert(0) += 1;
        }

        log!("{}:\n{:?}", name, counts);
    }


    #[test]
    fn test_bool() {
        test_distribution("bool", || bool());
    }

    #[test]
    fn test_percentage() {
        percentage();
        //test_distribution("percentage", || percentage());
    }
}
