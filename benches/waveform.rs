use std::hint;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rand::{RngExt, distr::Uniform};
use taudio::{
    Real,
    waveform::{Saw, Sine, Square, Triangle, WaveSource},
};

fn wave(xs: &[Real], mut w: impl WaveSource) {
    for &x in xs {
        let y = w.sample(x);

        hint::black_box(y);
    }
}

fn create_test_input(num_samples: usize) -> Vec<Real> {
    let rng = rand::rng();
    let dist = Uniform::new(0.0, 1.0).unwrap();

    rng.sample_iter(dist).take(num_samples).collect()
}

fn waveform(c: &mut Criterion) {
    let input = create_test_input(44100);

    c.bench_with_input(
        BenchmarkId::new("sine", "44100-samples"),
        &input.as_slice(),
        |b, &i| {
            b.iter(|| wave(i, Sine));
        },
    );

    c.bench_with_input(
        BenchmarkId::new("square", "44100-samples"),
        &input.as_slice(),
        |b, &i| {
            b.iter(|| wave(i, Square));
        },
    );

    c.bench_with_input(
        BenchmarkId::new("triangle", "44100-samples"),
        &input.as_slice(),
        |b, &i| {
            b.iter(|| wave(i, Triangle));
        },
    );

    c.bench_with_input(
        BenchmarkId::new("saw", "44100-samples"),
        &input.as_slice(),
        |b, &i| {
            b.iter(|| wave(i, Saw));
        },
    );
}

criterion_group!(benches, waveform);
criterion_main!(benches);
