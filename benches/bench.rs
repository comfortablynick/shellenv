use criterion::{black_box, criterion_group, criterion_main, Criterion};
type Result = anyhow::Result<()>;

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
}

fn simple_env_var() -> Result {
    const TOML: &str = r#"
        [[env]]
        key = 'LANG'
        val = 'en_US.utf8'
        cat = 'system'
        desc = 'Locale setting'
        shell = ['bash']
            "#;
    let mut buf = Vec::new();
    let _ = parse_config(&TOML, &Shell::Bash, &mut buf)?;
    let output = String::from_utf8(buf)?;
    assert_eq!(output, "export LANG=en_US.utf8\n");
    Ok(())
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
