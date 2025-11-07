use yini::Parser;

fn main() {
    let data = r#"
# Config file
server {
    host "localhost"
    port 8080
    ssl true
    threads 4
}

database {
    connection "postgresql://localhost/mydb"
    pool_size 10
    timeout 30.5
}

users [
    {
        name "Alice"
        age 30
        roles ["admin" user]
    }
    {
        name "Bob"
        age 25
        roles ["user"]
    }
]

coordinates [
    (100 200)
    (300 400)
    (500 600)
]

status :active
mode :fullscreen
color :rgb(255 128 0)
"#;

    // Warmup
    for _ in 0..100 {
        let mut parser = Parser::new(data);
        let _ = parser.parse();
    }

    // Benchmark
    let iterations = 10000;
    let start = std::time::Instant::now();

    for _ in 0..iterations {
        let mut parser = Parser::new(data);
        let _ = parser.parse();
    }

    let elapsed = start.elapsed();
    let per_iter = elapsed / iterations;

    println!("Parsed {} iterations in {:?}", iterations, elapsed);
    println!("Average time per parse: {:?}", per_iter);
    println!("Parses per second: {:.0}", 1.0 / per_iter.as_secs_f64());
}
