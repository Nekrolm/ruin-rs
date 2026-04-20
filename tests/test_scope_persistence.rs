use ruin_rs::{Scope, Value, eval};
use hashbrown::HashMap;

fn main() {
    // Test 1: Simple assignment to a pre-existing variable
    println!("Test 1: Assignment persistence");
    let mut scope = Scope {
        variables: {
            let mut map = HashMap::new();
            map.insert("x".to_string(), Value::Int(10));
            map
        },
    };

    let script = "x = 42;";
    let result = eval(script, &mut scope);
    println!("  Result: {:?}", result);
    println!("  x before: 10, x after: {:?}", scope.variables.get("x"));
    assert_eq!(
        scope.variables.get("x"),
        Some(&Value::Int(42)),
        "x should be 42 after assignment"
    );

    // Test 2: Variable definition and mutation
    println!("\nTest 2: Variable definition and mutation persistence");
    let mut scope2 = Scope::default();
    let script2 = r#"
        let y : int = 10;
        y = y + 5;
    "#;
    let result2 = eval(script2, &mut scope2);
    println!("  Result: {:?}", result2);
    println!("  y value after eval: {:?}", scope2.variables.get("y"));
    assert_eq!(
        scope2.variables.get("y"),
        Some(&Value::Int(15)),
        "y should be 15 after mutation"
    );

    // Test 3: Multiple variables and their persistence
    println!("\nTest 3: Multiple variables persistence");
    let mut scope3 = Scope::default();
    let script3 = r#"
        let a : int = 5;
        let b : int = 10;
        let c : int = a + b;
    "#;
    let result3 = eval(script3, &mut scope3);
    println!("  Result: {:?}", result3);
    println!(
        "  a = {:?}, b = {:?}, c = {:?}",
        scope3.variables.get("a"),
        scope3.variables.get("b"),
        scope3.variables.get("c")
    );
    assert_eq!(scope3.variables.get("a"), Some(&Value::Int(5)));
    assert_eq!(scope3.variables.get("b"), Some(&Value::Int(10)));
    assert_eq!(scope3.variables.get("c"), Some(&Value::Int(15)));

    println!("\n✅ All scope persistence tests passed!");
}
