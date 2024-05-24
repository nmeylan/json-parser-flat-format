use typed_arena::Arena;

type PointerFragment = Vec<&'static str>;

fn concat_route<'a>(route: &PointerFragment, arena: &'a Arena<u8>) -> &'a str {
    let bytes = route.iter().flat_map(|s| s.as_bytes());
    let buffer = arena.alloc_extend(bytes.cloned());
    unsafe { std::str::from_utf8_unchecked_mut(buffer) }
}

struct ParseResult<'a> {
    res: Vec<&'a str>,
}
struct Parser{}
impl Parser {

    fn parse<'a>(&self, arena: &'a Arena<u8>) -> ParseResult<'a> {
        let routes = vec![
            vec!["/home", "/user", "/profile"],
            vec!["/settings", "/preferences"],
            vec!["/dashboard", "/stats"],
        ];

        let mut result = vec![];

        for _ in 0..1000000 {
            for route in routes.iter() {
                result.push(concat_route(&route, arena));
            }
        }
        ParseResult {
            res: result
        }
    }
}

fn concat_route1(route: &PointerFragment) -> String {
    let mut res = String::with_capacity(64);
    for p in route {
        res.push_str(p);
    }
    res
}

fn main() {
    let initial_capacity = 1024 * 1024; // Adjust based on your expected needs
    let arena = Arena::new();
    let parser = Parser {};
    let result = parser.parse(&arena);
    for r in result.res {
        // println!("{}", r);
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