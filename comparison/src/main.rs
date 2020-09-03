use std::mem::size_of;

fn show_thunderdome() {
    use thunderdome::Index;

    println!("thunderdome");
    println!("Size of Index: {}", size_of::<Index>());
    println!("Size of Option<Index>: {}", size_of::<Option<Index>>());
}

fn show_generational_arena() {
    use generational_arena::Index;

    println!("generational-arena");
    println!("Size of Index: {}", size_of::<Index>());
    println!("Size of Option<Index>: {}", size_of::<Option<Index>>());
}

fn show_slotmap() {
    use slotmap::DefaultKey;

    println!("slotmap");
    println!("Size of DefaultKey: {}", size_of::<DefaultKey>());
    println!(
        "Size of Option<DefaultKey>: {}",
        size_of::<Option<DefaultKey>>()
    );
}

fn show_slab() {
    println!("slab");
    println!("Size of usize: {}", size_of::<usize>());
    println!("Size of Option<usize>: {}", size_of::<Option<usize>>());
}

fn main() {
    show_thunderdome();
    println!();

    show_generational_arena();
    println!();

    show_slotmap();
    println!();

    show_slab();
}
