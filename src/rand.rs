use std::hash::Hasher;

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new() -> Self {
        let seed =
            std::hash::BuildHasher::build_hasher(&std::collections::hash_map::RandomState::new())
                .finish();
        Self { state: seed }
    }

    #[cfg(test)]
    pub fn with_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        assert!(slice.len() < usize::from(u16::MAX), "{}", slice.len());

        for i in 1..slice.len() {
            slice.swap(i, usize::from(self.u16(i as u16)));
        }
    }

    pub fn bool(&mut self) -> bool {
        self.next() % 2 != 0
    }

    pub fn u16(&mut self, max_inclusive: u16) -> u16 {
        assert!(max_inclusive != u16::MAX);
        (self.next() % u64::from(max_inclusive + 1)) as u16
    }

    fn next(&mut self) -> u64 {
        // Xorshift RNG
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;

        self.state = x;

        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shuffle() {
        let mut rng = Rng::with_seed(391348571);

        let mut s = (0..10).collect::<Vec<_>>();
        rng.shuffle(&mut s);

        assert_eq!(s, &[6, 3, 7, 8, 1, 4, 0, 2, 9, 5]);
    }

    #[test]
    fn bool() {
        let mut rng = Rng::with_seed(123456789);

        assert!(rng.bool());
        assert!(!rng.bool());
        assert!(!rng.bool());
        assert!(!rng.bool());
        assert!(!rng.bool());
        assert!(!rng.bool());
        assert!(rng.bool());
        assert!(!rng.bool());
        assert!(!rng.bool());
        assert!(!rng.bool());

        let mut rng = Rng::with_seed(987654321);

        assert!(!rng.bool());
        assert!(!rng.bool());
        assert!(rng.bool());
        assert!(!rng.bool());
        assert!(rng.bool());
        assert!(rng.bool());
        assert!(!rng.bool());
        assert!(rng.bool());
        assert!(rng.bool());
        assert!(rng.bool());
    }

    #[test]
    fn u16() {
        let mut rng = Rng::with_seed(234567891);

        assert_eq!(rng.u16(10), 2);
        assert_eq!(rng.u16(10), 6);
        assert_eq!(rng.u16(10), 2);
        assert_eq!(rng.u16(10), 2);
        assert_eq!(rng.u16(10), 5);
        assert_eq!(rng.u16(10), 6);
        assert_eq!(rng.u16(10), 2);
        assert_eq!(rng.u16(10), 4);
        assert_eq!(rng.u16(10), 2);
        assert_eq!(rng.u16(10), 2);

        let mut rng = Rng::with_seed(876543212);

        assert_eq!(rng.u16(10), 9);
        assert_eq!(rng.u16(10), 1);
        assert_eq!(rng.u16(10), 3);
        assert_eq!(rng.u16(10), 6);
        assert_eq!(rng.u16(10), 9);
        assert_eq!(rng.u16(10), 10);
        assert_eq!(rng.u16(10), 4);
        assert_eq!(rng.u16(10), 1);
        assert_eq!(rng.u16(10), 9);
        assert_eq!(rng.u16(10), 2);

        let mut rng = Rng::with_seed(2765438120);

        assert_eq!(rng.u16(3), 1);
        assert_eq!(rng.u16(3), 0);
        assert_eq!(rng.u16(3), 3);
        assert_eq!(rng.u16(3), 1);
        assert_eq!(rng.u16(3), 2);
        assert_eq!(rng.u16(3), 3);
        assert_eq!(rng.u16(3), 1);
        assert_eq!(rng.u16(3), 0);
        assert_eq!(rng.u16(3), 3);
        assert_eq!(rng.u16(3), 3);
        assert_eq!(rng.u16(3), 2);
    }
}
