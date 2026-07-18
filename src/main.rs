use delta_forge::DeltaForge;

fn main() -> anyhow::Result<()> {
    let mut df = DeltaForge::new();

    df.input("a");
    df.input("b");

    df.compute("sum", |deps| {
        let a = deps[0].and_then(|v| v.downcast_ref::<i64>()).copied().unwrap_or(0);
        let b = deps[1].and_then(|v| v.downcast_ref::<i64>()).copied().unwrap_or(0);
        Box::new(a + b)
    });
    df.add_dep("sum", "a")?;
    df.add_dep("sum", "b")?;

    df.compute("doubled", |deps| {
        let s = deps[0].and_then(|v| v.downcast_ref::<i64>()).copied().unwrap_or(0);
        Box::new(s * 2)
    });
    df.add_dep("doubled", "sum")?;

    df.set_input("a", 3_i64)?;
    df.set_input("b", 4_i64)?;
    df.topo_recompute()?;

    println!("a=3, b=4 → sum={}, doubled={}", df.get::<i64>("sum")?, df.get::<i64>("doubled")?);
    df.print_log();

    df.clear_log();
    df.set_input("b", 10_i64)?;
    df.topo_recompute()?;

    println!("\na=3, b=10 → sum={}, doubled={}", df.get::<i64>("sum")?, df.get::<i64>("doubled")?);
    df.print_log();
    Ok(())
}
