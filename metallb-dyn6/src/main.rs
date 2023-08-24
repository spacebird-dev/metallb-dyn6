use address_source::AddressSource;
use interface::InterfaceSource;

fn main() {
    let iface = InterfaceSource::new("eth0").unwrap();
    match iface.get() {
        Ok(a) => {
            dbg!(a);
        }
        Err(e) => {
            dbg!(e);
        }
    };
}
