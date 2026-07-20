use assetrail_lib::connectors::binance::{BinanceSigner, RequestWeight, SignedQuery};

fn main() {
    let _ = std::mem::size_of::<BinanceSigner>();
    let _ = std::mem::size_of::<RequestWeight>();
    let _ = std::mem::size_of::<SignedQuery>();
}
