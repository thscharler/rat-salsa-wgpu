pub fn main() {
    for b in unic_ucd::BlockIter::new() {
        println!("{:?},", b.name);
    }
}
