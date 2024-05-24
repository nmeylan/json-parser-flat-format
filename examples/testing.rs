use std::cell::RefCell;

type PointerFragment = Vec<&'static str>;

struct MemoryPool {
    buffer: Vec<u8>,
    position: usize,
}

impl MemoryPool {
    fn new(initial_capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(initial_capacity),
            position: 0,
        }
    }

    fn allocate(&mut self, size: usize) -> &mut [u8] {
        if self.position + size > self.buffer.capacity() {
            // Double the buffer capacity if needed
            let new_capacity = (self.buffer.capacity() + size).max(self.buffer.capacity() * 2);
            self.buffer.reserve(new_capacity - self.buffer.capacity());
        }

        // Ensure the buffer length matches the position
        if self.buffer.len() < self.position + size {
            unsafe {
                self.buffer.set_len(self.position + size);
            }
        }

        let allocation = &mut self.buffer[self.position..self.position + size];
        self.position += size;
        allocation
    }

    fn clear(&mut self) {
        self.position = 0;
    }
}

struct Concatenator {
    pool: RefCell<MemoryPool>,
}

impl Concatenator {
    fn new(initial_capacity: usize) -> Self {
        Self {
            pool: RefCell::new(MemoryPool::new(initial_capacity)),
        }
    }

    fn concat_route(&self, route: &PointerFragment) -> String {
        let mut pool = self.pool.borrow_mut();
        pool.clear();

        let total_length: usize = route.iter().map(|s| s.len()).sum();
        let buffer = pool.allocate(total_length);

        let mut current_position = 0;
        for p in route {
            let bytes = p.as_bytes();
            let len = bytes.len();
            buffer[current_position..current_position + len].copy_from_slice(bytes);
            current_position += len;
        }

        String::from_utf8_lossy(buffer).into_owned()
    }
}

fn main() {
    let initial_capacity = 64; // Adjust based on your expected needs
    let concatenator = Concatenator::new(initial_capacity);

    let routes = vec![
        vec!["/home", "/user", "/profile"],
        vec!["/settings", "/preferences"],
        vec!["/dashboard", "/stats"],
    ];

    for route in routes {
        let concatenated = concatenator.concat_route(&route);
        println!("{}", concatenated);
    }
}



fn swar() {
    let str = "{  \n \"b\"}".as_bytes();
    // let mask = 0x7b7b7b7b7b7b7b7b;
    let mask = 0x0101010101010101 * b'\n' as u64;
    let bytes = u64::from_le_bytes([str[0], str[1], str[2], str[3], str[4], str[5], str[6], str[7]]);
    let comparison = mask ^ bytes;
    let high_bit_mask1 = (((comparison >> 1) | 0x8080808080808080) - comparison) & 0x8080808080808080;
    println!("{}", comparison);
    println!("{}", high_bit_mask1);
    let position = if high_bit_mask1 == 0 {
        None
    } else {
        Some(high_bit_mask1.trailing_zeros() >> 3)
    };
    println!("Position of {{ {:?}", position);

    let i = high_bit_mask1 == 0 && str[0] & 0x01 * b'{' != 0x01 * b'{';
    println!("{}", i);
}