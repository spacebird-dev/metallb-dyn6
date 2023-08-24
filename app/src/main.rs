use interface::InterfaceSource;
use prefix_source::PrefixSource;

fn main() {
    let iface = InterfaceSource::new("eth0").unwrap();
    match iface.get() {
        Ok(a) => {
            dbg!(a.net.addr());
        }
        Err(e) => {
            dbg!(e);
        }
    };
}
