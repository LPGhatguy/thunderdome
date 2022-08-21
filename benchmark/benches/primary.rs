use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use rand::{seq::SliceRandom, thread_rng};

use thunderdome::Arena;

pub fn iter(c: &mut Criterion) {
    let mut arena = Arena::new();
    for i in 0..10_000 {
        arena.insert(i);
    }

    c.bench_function("iter 10k", |b| {
        b.iter(|| {
            for kv in arena.iter() {
                black_box(kv);
            }
        })
    });
}

pub fn insert(c: &mut Criterion) {
    let arena: Arena<u64> = Arena::new();

    c.bench_function("insert 10k", |b| {
        b.iter_batched_ref(
            || arena.clone(),
            |arena| {
                for i in 0..10_000 {
                    arena.insert(i);
                }
            },
            BatchSize::SmallInput,
        )
    });
}

pub fn get_random(c: &mut Criterion) {
    let mut arena: Arena<u64> = Arena::new();

    let mut keys = Vec::new();
    for i in 0..10_000 {
        keys.push(arena.insert(i));
    }
    keys.shuffle(&mut thread_rng());

    c.bench_function("get_random 10k", |b| {
        b.iter(|| {
            for &k in &keys {
                black_box(arena.get(k));
            }
        })
    });
}

pub fn remove_random(c: &mut Criterion) {
    let mut arena: Arena<u64> = Arena::new();

    let mut keys = Vec::new();
    for i in 0..10_000 {
        keys.push(arena.insert(i));
    }
    keys.shuffle(&mut thread_rng());

    c.bench_function("remove_random 10k", |b| {
        b.iter_batched_ref(
            || arena.clone(),
            |arena| {
                for &k in &keys {
                    black_box(arena.remove(k));
                }
            },
            BatchSize::SmallInput,
        )
    });
}

pub fn reinsert_random(c: &mut Criterion) {
    let mut arena: Arena<u64> = Arena::new();

    let mut keys = Vec::new();
    for i in 0..10_000 {
        keys.push(arena.insert(i));
    }

    keys.shuffle(&mut thread_rng());

    for key in keys {
        arena.remove(key);
    }

    c.bench_function("reinsert_random 10k", |b| {
        b.iter_batched_ref(
            || arena.clone(),
            |arena| {
                for i in 0..10_000 {
                    black_box(arena.insert(i));
                }
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    benches,
    iter,
    insert,
    get_random,
    remove_random,
    reinsert_random
);
criterion_main!(benches);
