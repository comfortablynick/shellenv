use criterion::{criterion_group, criterion_main, Criterion};
use shellenv::{config::parse_config, shell::Shell, util::Result};

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

fn bench_toml(c: &mut Criterion) {
    c.bench_function("simple_env_var", |b| b.iter(|| simple_env_var()));
}

criterion_group!(benches, bench_toml);
criterion_main!(benches);
