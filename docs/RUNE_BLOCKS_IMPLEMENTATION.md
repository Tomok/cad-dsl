# Rune Blocks Implementation Plan

## Document Overview

This document outlines the implementation plan for integrating Rune blocks into the CAD-DSL. Rune blocks enable imperative computations within the declarative constraint-based language.

**Status**: Planning Phase
**Start Date**: 2026-01-23
**Target Completion**: TBD

---

## Design Decisions

### Q1: Execution Timing
**Decision**: Execute when all parameters are known (like for-loops)
**Rationale**: Simplest approach, clear semantics, no circular dependencies

### Q2: Constraint Direction
**Decision**: One-way data flow - parameters → rune → result → constraints on other variables
**Example**:
```rust
let x: f64;
x > 0.0;  // x must be fully determined

let y = rune(x) { x * x };  // y computed from x

let z: f64;
z == y + 10.0;  // y can constrain z ✅

// INVALID: y cannot backward-constrain x
y < 100.0;  // ERROR: x must determine independently
```

### Q3: Parser Strategy
**Decision**: Capture rune body as string with bracket counting
**Rationale**: Simplest approach, delegates parsing to Rune, allows nested braces

### Q4: Type System
**Decision**: Implicit type inference
**Rationale**: Matches Rune's philosophy, cleaner syntax

### Parameter Syntax Extension
**Decision**: Support both direct parameters and assignments
```rust
rune(x)              // Direct: x = x
rune(x=p.x)          // Rename: x = p.x
rune(x=p.x, y, z=100) // Mixed: rename, direct, constant
```

---

## Implementation Phases

### Phase 0: Documentation & Planning ✅
**Status**: Complete

- [x] Update language specification (TEXTCAD_LANGUAGE_SPEC.md)
- [x] Create implementation plan document (this file)
- [x] Update CLAUDE.md with rune block status

### Phase 1: Parser & AST
**Estimated Effort**: 2-3 days
**Status**: Not Started

#### Step 1.1: Add Rune Dependency
```toml
# Cargo.toml
[dependencies]
rune = "0.14"
rune-alloc = "0.14"
```

#### Step 1.2: Lexer Extension
```rust
// src/lexer.rs
#[token("rune")]
Rune,
```

#### Step 1.3: AST Extension
```rust
// src/ast/expr.rs (new structures)
pub struct RuneParam<'src> {
    pub name: &'src str,           // Parameter name in rune code
    pub value: Option<Expr<'src>>, // Optional: expression to bind
    pub span: Span,
}

pub struct RuneBlock<'src> {
    pub params: Vec<RuneParam<'src>>,
    pub body: &'src str,  // Raw Rune code
    pub span: Span,
}

// Add to Atom enum
pub enum Atom<'src> {
    // ... existing variants
    RuneBlock(Box<RuneBlock<'src>>),
}
```

#### Step 1.4: Parser Implementation
```rust
// src/parser/expr.rs
// Grammar: rune(params) { body }
//   params: param | param = expr | param, params
//   body: arbitrary Rune code with balanced braces

fn rune_block_parser() -> impl Parser<...> {
    just(Token::Rune)
        .ignore_then(rune_params())
        .then(rune_body())
        .map(|(params, body)| {
            Atom::RuneBlock(Box::new(RuneBlock {
                params,
                body,
                span,
            }))
        })
}

fn rune_params() -> impl Parser<...> {
    // Parse: (x) or (x, y) or (x=expr, y, z=100)
    let param = ident()
        .then(just(Token::Assign).ignore_then(expr()).or_not())
        .map(|(name, value)| RuneParam { name, value, span });

    param
        .separated_by(just(Token::Comma))
        .collect::<Vec<_>>()
        .delimited_by(just(Token::LParen), just(Token::RParen))
}

fn rune_body() -> impl Parser<...> {
    // Capture body as string with bracket counting
    // Must handle nested { } correctly

    // Implementation strategy:
    // 1. After seeing {, count brace depth
    // 2. Accumulate all tokens until matching }
    // 3. Convert tokens back to string

    // TODO: Detailed implementation
}
```

**Deliverables**:
- Rune token in lexer
- AST nodes for RuneBlock and RuneParam
- Parser for rune blocks with bracket counting
- Unit tests for parser

**Tests**:
```rust
// Basic syntax
rune(x) { x * 2 }

// Multiple parameters
rune(x, y) { x + y }

// Parameter assignments
rune(x=p.x, y=p.y) { x * x + y * y }

// Mixed parameters
rune(x=p.x, y, z=100) { x + y + z }

// Nested braces
rune(x) {
    let y = { x + 5 };
    if y > 10 { y * 2 } else { y }
}

// Complex expressions
rune(x=points[0].x * 2.0, y) { x + y }
```

---

### Phase 2: Semantic Analysis & HIR
**Estimated Effort**: 2-3 days
**Status**: Not Started

#### Step 2.1: HIR Extension
```rust
// src/hir/expr.rs
pub struct ResolvedRuneParam<'src, 'arena> {
    pub name: &'src str,
    pub value: &'arena ResolvedExpr<'src, 'arena>,  // Resolved expression
    pub span: Span,
}

pub struct ResolvedRuneBlock<'src, 'arena> {
    pub params: Vec<ResolvedRuneParam<'src, 'arena>>,
    pub body: &'src str,
    pub return_type: ResolvedType<'src, 'arena>,  // Inferred by type checker
    pub span: Span,
}

// Add to ResolvedExprKind
pub enum ResolvedExprKind<'src, 'arena> {
    // ... existing variants
    RuneBlock(ResolvedRuneBlock<'src, 'arena>),
}
```

#### Step 2.2: Semantic Analyzer Extension
```rust
// src/semantic_analyzer/expr_resolver.rs
impl<'src, 'arena> ExprResolver<'src, 'arena> {
    fn resolve_rune_block(
        &mut self,
        rune_block: &RuneBlock<'src>,
    ) -> Result<&'arena ResolvedExpr<'src, 'arena>, SemanticError> {
        // 1. Resolve parameter expressions
        let params: Vec<_> = rune_block.params
            .iter()
            .map(|param| {
                let value = if let Some(expr) = &param.value {
                    self.resolve_expr(expr)?
                } else {
                    // Direct parameter: lookup variable
                    let var = self.scope.lookup_var(param.name)?;
                    self.arena.alloc(ResolvedExpr {
                        kind: ResolvedExprKind::VarRef(var),
                        type_: var.type_,
                        span: param.span,
                    })
                };

                Ok(ResolvedRuneParam {
                    name: param.name,
                    value,
                    span: param.span,
                })
            })
            .collect::<Result<_, _>>()?;

        // 2. Return type will be inferred by type checker
        let return_type = ResolvedType::Unknown;

        // 3. Create resolved rune block
        Ok(self.arena.alloc(ResolvedExpr {
            kind: ResolvedExprKind::RuneBlock(ResolvedRuneBlock {
                params,
                body: rune_block.body,
                return_type,
                span: rune_block.span,
            }),
            type_: return_type,
            span: rune_block.span,
        }))
    }
}
```

**Deliverables**:
- HIR nodes for resolved rune blocks
- Semantic analyzer resolution
- Variable lookup and expression resolution
- Unit tests

**Tests**:
- Parameter resolution (direct and assigned)
- Variable lookup in parameters
- Field access in parameters
- Scope handling
- Error cases (undefined variables, etc.)

---

### Phase 3: Type Checking & Rune Integration
**Estimated Effort**: 3-4 days
**Status**: Not Started

#### Step 3.1: Rune Type Checking Integration
```rust
// src/type_checker/rune_integration.rs (NEW FILE)
use rune::{Context, Diagnostics, Source, Sources};
use rune::compile::FileSourceLoader;

pub struct RuneTypeChecker {
    context: Context,
}

impl RuneTypeChecker {
    pub fn new() -> Result<Self, Error> {
        let mut context = rune::Context::with_default_modules()?;

        // Register additional modules
        context.install(rune_modules::core::module(true)?)?;
        context.install(rune_modules::io::module(true)?)?;

        Ok(Self { context })
    }

    pub fn infer_type(
        &self,
        rune_block: &ResolvedRuneBlock,
        param_types: &[ResolvedType],
    ) -> Result<ResolvedType, TypeError> {
        // 1. Generate Rune function wrapper
        let rune_code = self.generate_rune_function(rune_block, param_types)?;

        // 2. Compile to check types
        let mut sources = Sources::new();
        sources.insert(Source::new("rune_block", rune_code)?)?;

        let mut diagnostics = Diagnostics::new();
        let result = rune::prepare(&mut sources)
            .with_context(&self.context)
            .with_diagnostics(&mut diagnostics)
            .build();

        // 3. Check for compile errors
        if !diagnostics.is_empty() {
            return Err(TypeError::RuneCompileError(diagnostics));
        }

        let unit = result?;

        // 4. Extract return type from unit metadata
        // TODO: Use Rune's reflection API
        let return_type = self.extract_return_type(&unit)?;

        Ok(return_type)
    }

    fn generate_rune_function(
        &self,
        rune_block: &ResolvedRuneBlock,
        param_types: &[ResolvedType],
    ) -> Result<String, Error> {
        let mut code = String::from("pub fn __rune_fn__(");

        // Add parameters with types
        for (i, (param, ty)) in rune_block.params.iter().zip(param_types).enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            code.push_str(param.name);
            code.push_str(": ");
            code.push_str(&self.map_type_to_rune(ty)?);
        }

        code.push_str(") {\n");
        code.push_str(rune_block.body);
        code.push_str("\n}");

        Ok(code)
    }

    fn map_type_to_rune(&self, ty: &ResolvedType) -> Result<String, Error> {
        match ty {
            ResolvedType::I32 => Ok("i64".to_string()),  // Rune uses i64
            ResolvedType::F64 => Ok("f64".to_string()),
            ResolvedType::Bool => Ok("bool".to_string()),
            // TODO: Handle structs, arrays, etc.
            _ => Err(Error::UnsupportedType(ty.clone())),
        }
    }

    fn extract_return_type(&self, unit: &Unit) -> Result<ResolvedType, Error> {
        // Use Rune's reflection API to get return type
        // TODO: Implementation depends on Rune API

        // Placeholder:
        Ok(ResolvedType::F64)
    }
}
```

#### Step 3.2: Type Checker Extension
```rust
// src/type_checker/inference.rs
impl<'src, 'arena> TypeInference<'src, 'arena> {
    fn infer_rune_block(
        &mut self,
        rune_block: &ResolvedRuneBlock<'src, 'arena>,
    ) -> Result<ResolvedType<'src, 'arena>, TypeError> {
        // 1. Get parameter types
        let param_types: Vec<_> = rune_block.params
            .iter()
            .map(|p| self.infer_expr(p.value))
            .collect::<Result<_, _>>()?;

        // 2. Use Rune type checker
        let rune_checker = RuneTypeChecker::new()?;
        let return_type = rune_checker.infer_type(rune_block, &param_types)?;

        Ok(return_type)
    }
}
```

**Deliverables**:
- Rune type checking integration
- Type mapping CAD-DSL ↔ Rune
- Return type inference
- Error handling and reporting
- Unit tests

**Type Mappings**:
```
CAD-DSL     → Rune
---------     ----
i32         → i64
f64         → f64
bool        → bool
String      → String
Struct      → Object/Struct (TODO)
[T; N]      → Vec<T> (TODO)
```

---

### Phase 4: Solver Integration & Execution
**Estimated Effort**: 3-4 days
**Status**: Not Started

#### Step 4.1: Rune Executor
```rust
// src/solver/rune_executor.rs (NEW FILE)
use rune::{Context, Unit, Vm, Value};
use std::sync::Arc;

pub struct RuneExecutor {
    context: Arc<Context>,
}

impl RuneExecutor {
    pub fn new() -> Result<Self, Error> {
        let mut context = Context::with_default_modules()?;

        // Register modules
        context.install(rune_modules::core::module(true)?)?;
        context.install(rune_modules::io::module(true)?)?;

        Ok(Self {
            context: Arc::new(context),
        })
    }

    pub fn execute_block(
        &self,
        rune_block: &ResolvedRuneBlock,
        param_values: Vec<Value>,
    ) -> Result<Value, Error> {
        // 1. Generate Rune function
        let rune_code = self.generate_rune_code(rune_block)?;

        // 2. Compile
        let mut sources = Sources::new();
        sources.insert(Source::new("rune_block", rune_code)?)?;

        let mut diagnostics = Diagnostics::new();
        let unit = rune::prepare(&mut sources)
            .with_context(&*self.context)
            .with_diagnostics(&mut diagnostics)
            .build()?;

        if !diagnostics.is_empty() {
            return Err(Error::RuneCompileError(diagnostics));
        }

        // 3. Execute
        let unit = Arc::new(unit);
        let mut vm = Vm::new(Arc::clone(&self.context), unit);

        let result = vm.call(["__rune_fn__"], param_values)?;

        Ok(result)
    }

    fn generate_rune_code(&self, rune_block: &ResolvedRuneBlock) -> Result<String, Error> {
        let mut code = String::from("pub fn __rune_fn__(");

        // Add parameters (types inferred by Rune)
        for (i, param) in rune_block.params.iter().enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            code.push_str(param.name);
        }

        code.push_str(") {\n");
        code.push_str(rune_block.body);
        code.push_str("\n}");

        Ok(code)
    }

    pub fn convert_to_rune_value(&self, value: &Z3Value, ty: &ResolvedType) -> Result<Value, Error> {
        match (value, ty) {
            (Z3Value::Int(i), ResolvedType::I32) => Ok(Value::from(*i as i64)),
            (Z3Value::Real(f), ResolvedType::F64) => Ok(Value::from(*f)),
            (Z3Value::Bool(b), ResolvedType::Bool) => Ok(Value::from(*b)),
            // TODO: Structs, arrays
            _ => Err(Error::TypeConversionError),
        }
    }

    pub fn convert_from_rune_value(&self, value: Value, ty: &ResolvedType) -> Result<Z3Value, Error> {
        match ty {
            ResolvedType::I32 => {
                let i: i64 = rune::from_value(value)?;
                Ok(Z3Value::Int(i as i32))
            }
            ResolvedType::F64 => {
                let f: f64 = rune::from_value(value)?;
                Ok(Z3Value::Real(f))
            }
            ResolvedType::Bool => {
                let b: bool = rune::from_value(value)?;
                Ok(Z3Value::Bool(b))
            }
            // TODO: Structs, arrays
            _ => Err(Error::TypeConversionError),
        }
    }
}
```

#### Step 4.2: Solver Pipeline Extension
```rust
// src/solver/mod.rs
pub fn solve_with_rune(
    hir: &HirProgram,
) -> Result<Solution, SolverError> {
    // 1. Extract constraints (excluding rune blocks)
    let (constraints, rune_blocks) = extract_constraints_and_rune_blocks(hir)?;

    // 2. Validate dependency order
    validate_rune_dependencies(&constraints, &rune_blocks)?;

    // 3. Solve constraints (Z3)
    let z3_solution = solve_z3_constraints(constraints)?;

    // 4. Execute rune blocks
    let rune_executor = RuneExecutor::new()?;
    let rune_results = execute_rune_blocks_in_order(
        &rune_blocks,
        &z3_solution,
        &rune_executor,
    )?;

    // 5. Merge solutions
    let final_solution = merge_solutions(z3_solution, rune_results);

    Ok(final_solution)
}

fn extract_constraints_and_rune_blocks(
    hir: &HirProgram,
) -> Result<(Vec<Constraint>, Vec<RuneBlockInfo>), Error> {
    // Walk HIR and separate regular constraints from rune blocks
    // TODO: Implementation
}

fn validate_rune_dependencies(
    constraints: &[Constraint],
    rune_blocks: &[RuneBlockInfo],
) -> Result<(), Error> {
    // Check that:
    // 1. All rune parameters can be determined before execution
    // 2. No circular dependencies
    // 3. Rune results don't backward-constrain parameters

    // TODO: Implementation
}

fn execute_rune_blocks_in_order(
    rune_blocks: &[RuneBlockInfo],
    z3_solution: &Z3Solution,
    executor: &RuneExecutor,
) -> Result<HashMap<VarId, Value>, Error> {
    let mut results = HashMap::new();

    for block in rune_blocks {
        // 1. Get parameter values from Z3 solution
        let param_values: Vec<_> = block.params
            .iter()
            .map(|p| {
                let z3_value = z3_solution.get(&p.var_id)?;
                executor.convert_to_rune_value(z3_value, &p.type_)
            })
            .collect::<Result<_, _>>()?;

        // 2. Execute rune block
        let result = executor.execute_block(&block.rune_block, param_values)?;

        // 3. Convert result back to solver value
        let solver_value = executor.convert_from_rune_value(result, &block.return_type)?;

        // 4. Store result
        results.insert(block.result_var_id, solver_value);
    }

    Ok(results)
}
```

**Deliverables**:
- Rune executor for running compiled blocks
- Solver integration (post-processing after Z3)
- Dependency validation
- Value conversion CAD-DSL ↔ Rune
- Integration tests

**Tests**:
- Simple arithmetic: `rune(x) { x * 2 }`
- Multiple parameters: `rune(x, y) { x + y }`
- Control flow: `rune(x) { if x > 0 { x } else { -x } }`
- Loops: `rune(n) { let mut sum = 0; for i in 0..n { sum += i; } sum }`
- Struct return: `rune(x, y) { Point { x, y } }`
- Using results in constraints

---

### Phase 5: Testing & Documentation
**Estimated Effort**: 2 days
**Status**: Not Started

#### Step 5.1: Comprehensive Tests
- Unit tests for each component
- Integration tests (end-to-end)
- Error case tests
- Performance tests

#### Step 5.2: Documentation
- Update CLAUDE.md implementation status
- Add examples to language spec
- Create user guide for rune blocks
- Document limitations and gotchas

#### Step 5.3: Example Programs
Create comprehensive example programs:
- Fibonacci calculation
- Polar to Cartesian conversion
- Triangle area calculation (Heron's formula)
- Newton's method iteration
- Complex geometric transformations

**Deliverables**:
- Full test suite
- Updated documentation
- Example programs
- User guide

---

## Future Enhancements (Phase 6+)

### Phase 6: Advanced Type Support
- Struct parameter/return types
- Array parameter/return types
- Custom type conversions

### Phase 7: Standard Library Integration
- Math module (sin, cos, sqrt, etc.)
- Geometry helpers
- Array utilities

### Phase 8: Bidirectional Constraints (Advanced)
- Allow rune results in constraints that influence parameters
- Requires iterative solving
- Z3 User Propagator integration
- **Warning**: Very complex, may not be feasible

### Phase 9: Performance Optimizations
- Rune code caching (compile once, execute multiple times)
- Parallel execution of independent rune blocks
- JIT compilation exploration

### Phase 10: Debugging Support
- Error message improvements
- Source mapping for stack traces
- Debug output/logging from rune blocks
- Integration with IDE debugging tools

---

## Technical Challenges

### Challenge 1: Parser Complexity
**Problem**: Parsing rune body with nested braces
**Solution**: Bracket counting in parser, capture as string

### Challenge 2: Type System Mismatch
**Problem**: CAD-DSL has i32, Rune has i64
**Solution**: Explicit conversions, map types at boundary

### Challenge 3: Error Reporting
**Problem**: Errors in rune code need clear messages
**Solution**: Map Rune error spans to CAD-DSL source locations

### Challenge 4: Execution Timing
**Problem**: When to execute rune blocks in solver pipeline?
**Solution**: Post-processing after constraint solving

### Challenge 5: Dependency Validation
**Problem**: Ensuring parameters are fully determined
**Solution**: Static analysis of constraint graph

### Challenge 6: Value Conversion
**Problem**: Converting between solver values and Rune values
**Solution**: Type-safe conversion functions with error handling

---

## Success Criteria

### Minimum Viable Product (MVP)
- [ ] Parser recognizes rune blocks
- [ ] Type checker validates rune code
- [ ] Solver executes rune blocks after constraints
- [ ] Basic types work (i32, f64, bool)
- [ ] Results can be used in further constraints
- [ ] Error messages are clear

### Full Feature Set
- [ ] All primitive types supported
- [ ] Struct parameters and returns
- [ ] Array parameters and returns
- [ ] Parameter assignment syntax
- [ ] Comprehensive error handling
- [ ] Performance is acceptable
- [ ] Documentation complete
- [ ] Test coverage >80%

---

## Timeline Estimate

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| Phase 0: Documentation | 0.5 days | None |
| Phase 1: Parser & AST | 2-3 days | Phase 0 |
| Phase 2: Semantic Analysis | 2-3 days | Phase 1 |
| Phase 3: Type Checking | 3-4 days | Phase 2 |
| Phase 4: Solver Integration | 3-4 days | Phase 3 |
| Phase 5: Testing & Docs | 2 days | Phase 4 |
| **Total** | **12-16 days** | |

---

## Risk Assessment

| Risk | Severity | Likelihood | Mitigation |
|------|----------|------------|------------|
| Rune API changes | Medium | Low | Pin Rune version, test upgrades carefully |
| Type system complexity | High | Medium | Start with basic types, add gradually |
| Performance overhead | Medium | Medium | Cache compiled code, profile early |
| Parser edge cases | Medium | High | Comprehensive test suite |
| Circular dependencies | High | Low | Static validation, clear error messages |
| User confusion | Medium | Medium | Clear documentation, good examples |

---

## References

- [Rune Language Documentation](https://rune-rs.github.io/)
- [Rune GitHub Repository](https://github.com/rune-rs/rune)
- [CAD-DSL Language Spec](./TEXTCAD_LANGUAGE_SPEC.md)
- [CAD-DSL Solver Architecture](./SOLVER_ARCHITECTURE.md)

---

**Document Version**: 1.0
**Last Updated**: 2026-01-23
**Author**: Claude Code Implementation Team
