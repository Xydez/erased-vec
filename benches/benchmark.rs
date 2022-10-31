use criterion::{black_box, criterion_group, criterion_main, Criterion};
use erased_vec::ErasedVec;
use rand::Rng;

fn bench_erasedvec(criterion: &mut Criterion) {
	criterion.bench_function("erasedvec-with_capacity", |bencher| {
		bencher.iter(|| {
			let _vec = ErasedVec::with_capacity::<i32>(black_box(512));
		});
	});

	criterion.bench_function("erasedvec-push", |bencher| {
    	let mut vec = ErasedVec::new::<i32>();

    	bencher.iter(|| {
			vec.push::<i32>(black_box(42));
    	});
	});

	criterion.bench_function("erasedvec-remove", |bencher| {
		// Generate a random ErasedVec of size 32..1024 with random data
		let mut vec = ErasedVec::new::<i32>();
		let mut rng = rand::thread_rng();

		for _ in 0..rng.gen_range(32..1024) {
			vec.push::<i32>(rng.gen());
		}

		bencher.iter_batched(|| vec.clone(), |mut vec| {
			let _v = vec.remove::<i32>(0);
		}, criterion::BatchSize::LargeInput);
	});

	criterion.bench_function("erasedvec-erase", |bencher| {
		// Generate a random ErasedVec of size 32..1024 with random data
		let mut vec = ErasedVec::new::<i32>();
		let mut rng = rand::thread_rng();

		for _ in 0..rng.gen_range(32..1024) {
			vec.push::<i32>(rng.gen());
		}

		bencher.iter_batched(|| vec.clone(), |mut vec| {
			vec.erase(0);
		}, criterion::BatchSize::LargeInput);
	});

}

fn bench_vec(criterion: &mut Criterion) {
	criterion.bench_function("vec-with_capacity", |bencher| {
		bencher.iter(|| {
			let _vec = ErasedVec::with_capacity::<i32>(black_box(512));
		});
	});

	criterion.bench_function("vec-push", |bencher| {
    	let mut vec = ErasedVec::new::<i32>();

    	bencher.iter(|| {
			vec.push::<i32>(black_box(42));
    	});
	});

	criterion.bench_function("vec-remove", |bencher| {
		// Generate a random Vec of size 32..1024 with random data
		let mut vec = Vec::<i32>::new();
		let mut rng = rand::thread_rng();

		for _ in 0..rng.gen_range(32..1024) {
			vec.push(rng.gen());
		}

		bencher.iter_batched(|| vec.clone(), |mut vec| {
			let _v = vec.remove(0);
		}, criterion::BatchSize::LargeInput);
	});

}

criterion_group!(benches, bench_erasedvec, bench_vec);
criterion_main!(benches);
