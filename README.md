# Parent and Child cell

A cell of a parent and a child, which is created by mutably borrowing the parent.
While the parent is in the cell, it cannot be accessed in any way.
Provides mutable access to the child.

This is useful in a rare case when you need to store and move both
parent and their child together.

## Example

Basic usage:
```rust
struct Hello {
    world: i64,
}
let hello = Hello { world: 10 };

let mut pac = pac_cell::PacCell::new(hello, |h| &mut h.world);

let initial = pac.with_mut(|world| {
    let i = **world;
    **world = 12;
    i
});
assert_eq!(initial, 10);

let hello_again = pac.unwrap();
assert_eq!(hello_again.world, 12);
```

For a real-world-like example, see the [crate tests](https://github.com/aljazerzen/pac_cell/blob/main/tests/it/main.rs).

## Soundness

This crate is fully sound and "incorrect" usage might lead to undefined behavior.
See https://users.rust-lang.org/t/soundness-of-pac-cell-library/108598/4
