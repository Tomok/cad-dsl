# Partial Solve Design for Phase 3

## Overview

This document describes the **iterative partial solving** architecture for Phase 3 of the solver migration. The goal is to enable the solver to handle constraints that depend on values that are initially unknown but can be determined through solving.

## Motivation

### The Problem

Consider this CAD-DSL program:

```
let n: i32;
let points: [Point; 10];

// This constraint is solvable immediately
n * 2 == 10;  // → n = 5

// This loop cannot be unrolled until n is known
for i in 0..n {
    points[i].x == i * 10;
    points[i].y == i * 20;
}
```

**Current behavior**: Solver fails because `n` is unknown when trying to unroll the loop.

**Desired behavior**:
1. Solve `n * 2 == 10` first → `n = 5`
2. With `n` known, unroll the loop and solve the remaining constraints
3. Return complete solution

### Use Cases

1. **For loops with computed ranges**: `for i in 0..n` where `n` is constrained elsewhere
2. **Array accesses with computed indices**: `arr[computed_index]` where index depends on other variables
3. **Conditional function calls**: Function calls whose resolution depends on solved variables
4. **Cascading dependencies**: Multi-level dependencies where solving one constraint enables solving others

## Architecture

### Core Concept: Deferred Constraints

A **deferred constraint** is a constraint that cannot be immediately processed because it depends on unknown values. The solver:

1. **Identifies** which constraints can be deferred (e.g., for-loops, dynamic array access)
2. **Extracts dependencies** (which variables must be known to proceed)
3. **Solves immediately resolvable constraints** first
4. **Re-evaluates deferred constraints** after each solve iteration
5. **Iterates** until all constraints are resolved or no progress is made

### API Design

```rust
/// Result of a solve operation
pub enum SolveResult<'src, 'arena> {
    /// All constraints were fully resolved
    Complete {
        solution: Solution<'src, 'arena>,
        iterations: usize,
    },

    /// Partial solution - some constraints could not be resolved
    Partial {
        solution: Solution<'src, 'arena>,
        deferred: Vec<DeferredConstraint<'src, 'arena>>,
        reason: PartialReason,
        iterations: usize,
    },
}

impl<'src, 'arena> SolveResult<'src, 'arena> {
    /// Check if the solve was complete (all constraints resolved)
    pub fn is_complete(&self) -> bool {
        matches!(self, SolveResult::Complete { .. })
    }

    /// Get the solution (works for both complete and partial)
    pub fn solution(&self) -> &Solution<'src, 'arena> {
        match self {
            SolveResult::Complete { solution, .. } => solution,
            SolveResult::Partial { solution, .. } => solution,
        }
    }

    /// Get number of iterations performed
    pub fn iterations(&self) -> usize {
        match self {
            SolveResult::Complete { iterations, .. } => *iterations,
            SolveResult::Partial { iterations, .. } => *iterations,
        }
    }
}

/// Reason why solving was only partial
#[derive(Debug, Clone, PartialEq)]
pub enum PartialReason {
    /// For-loop with unresolved range variable
    UnknownLoopRange {
        range_var: String,
        loop_span: Span,
    },

    /// Array index could not be evaluated to constant
    UnresolvedArrayIndex {
        index_expr: String,
        span: Span,
    },

    /// Function call with unresolved dependencies
    UnresolvedFunctionCall {
        function_name: String,
        missing_deps: Vec<String>,
        span: Span,
    },

    /// Maximum iterations reached without full resolution
    MaxIterationsReached {
        max: usize,
        remaining_deferred: usize,
    },

    /// No progress made - deferred constraints still have unknown dependencies
    NoProgress {
        stuck_constraints: Vec<String>,
    },
}

/// A constraint that has been deferred for later resolution
#[derive(Debug, Clone)]
pub struct DeferredConstraint<'src, 'arena> {
    /// The original HIR statement that couldn't be processed
    pub stmt: &'arena ResolvedStmt<'src, 'arena>,

    /// Variables that must have known values to process this constraint
    pub dependencies: Vec<&'src str>,

    /// Human-readable description of what's being deferred
    pub description: String,

    /// Span for error reporting
    pub span: Span,
}

/// Solution containing variable assignments from Z3
#[derive(Debug, Clone)]
pub struct Solution<'src, 'arena> {
    /// Map from variable path to concrete value
    pub assignments: HashMap<VariablePath<'src>, Value>,

    /// Variables that exist but have no determined value yet
    pub undetermined: Vec<VariablePath<'src>>,
}

/// A concrete value from the Z3 model
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Real(f64),
    Bool(bool),
}
```

### Solver Context Extensions

The `SolverContext` needs to track deferred constraints and iteration state:

```rust
pub struct SolverContext<'src, 'arena> {
    // ... existing fields ...

    /// Constraints that have been deferred
    deferred_constraints: Vec<DeferredConstraint<'src, 'arena>>,

    /// Current iteration number
    iteration: usize,

    /// Maximum iterations before giving up
    max_iterations: usize,

    /// Solution from the last Z3 solve (if any)
    current_solution: Option<Solution<'src, 'arena>>,
}

impl<'src, 'arena> SolverContext<'src, 'arena> {
    /// Defer a constraint for later resolution
    pub fn defer_constraint(
        &mut self,
        stmt: &'arena ResolvedStmt<'src, 'arena>,
        dependencies: Vec<&'src str>,
        description: String,
    ) {
        self.deferred_constraints.push(DeferredConstraint {
            stmt,
            dependencies,
            description,
            span: stmt.span,
        });
    }

    /// Check if a variable has a known value in the current solution
    pub fn is_variable_known(&self, var: &str) -> bool {
        if let Some(solution) = &self.current_solution {
            let path = VariablePath::from_name(var);
            solution.assignments.contains_key(&path)
        } else {
            false
        }
    }

    /// Get the value of a variable from the current solution
    pub fn get_variable_value(&self, var: &str) -> Option<&Value> {
        self.current_solution.as_ref().and_then(|sol| {
            let path = VariablePath::from_name(var);
            sol.assignments.get(&path)
        })
    }

    /// Check which deferred constraints can now be processed
    pub fn ready_deferred_constraints(&self) -> Vec<&DeferredConstraint<'src, 'arena>> {
        self.deferred_constraints
            .iter()
            .filter(|dc| {
                // Check if all dependencies are now known
                dc.dependencies.iter().all(|dep| self.is_variable_known(dep))
            })
            .collect()
    }
}
```

### Solvable Trait Extension

The `Solvable` trait needs to support deferral:

```rust
/// Result of attempting to solve a node
pub enum SolveOutcome<T> {
    /// Successfully solved - constraints added to Z3
    Solved(T),

    /// Deferred - dependencies not yet known
    Deferred {
        dependencies: Vec<&'src str>,
        description: String,
    },
}

pub trait Solvable<'src, 'arena> {
    type Output;

    /// Attempt to solve this node
    ///
    /// Returns:
    /// - `Ok(Solved(output))` if successful
    /// - `Ok(Deferred { ... })` if dependencies are missing
    /// - `Err(...)` for unrecoverable errors
    fn solve(
        &self,
        ctx: &mut SolverContext<'src, 'arena>
    ) -> Result<SolveOutcome<Self::Output>, SolverError>;
}
```

## Implementation Strategy

### Phase 3a: Basic Iterative Solving (No Deferral Yet)

**Goal**: Get the iterative solve loop working with simple constraints

1. **Implement `Solution` extraction from Z3 model**
   - Read all variables from Z3 model after solving
   - Store in `Solution` struct

2. **Implement iterative solve loop**
   ```rust
   pub fn solve_iterative<'src, 'arena>(
       statements: &[&'arena ResolvedStmt<'src, 'arena>],
       ctx: &mut SolverContext<'src, 'arena>,
   ) -> Result<SolveResult<'src, 'arena>, SolverError> {
       for iteration in 0..ctx.max_iterations {
           ctx.iteration = iteration;

           // Try to solve all statements
           for stmt in statements {
               stmt.solve(ctx)?;
           }

           // Run Z3 solver
           match ctx.z3_solver.check() {
               z3::SatResult::Sat => {
                   let solution = extract_solution(ctx)?;
                   ctx.current_solution = Some(solution.clone());

                   // Check if we have deferred constraints
                   if ctx.deferred_constraints.is_empty() {
                       return Ok(SolveResult::Complete {
                           solution,
                           iterations: iteration + 1,
                       });
                   }

                   // Try to process deferred constraints in next iteration
                   continue;
               }
               z3::SatResult::Unsat => {
                   return Err(SolverError::Unsatisfiable);
               }
               z3::SatResult::Unknown => {
                   return Err(SolverError::Unknown);
               }
           }
       }

       // Max iterations reached
       Err(SolverError::MaxIterationsReached)
   }
   ```

3. **Test with simple multi-iteration scenarios**
   - Variables with initializers
   - Cascading constraints

### Phase 3b: For-Loop Deferral

**Goal**: Defer for-loops with unknown ranges

1. **Implement `impl Solvable for ForLoop`**
   ```rust
   impl<'src, 'arena> Solvable<'src, 'arena> for ResolvedStmt<'src, 'arena> {
       type Output = ();

       fn solve(
           &self,
           ctx: &mut SolverContext<'src, 'arena>
       ) -> Result<SolveOutcome<()>, SolverError> {
           match &self.kind {
               ResolvedStmtKind::For { range, body, .. } => {
                   // Try to evaluate range bounds
                   match evaluate_range_bounds(range, ctx) {
                       Ok((start, end)) => {
                           // Range is known - unroll loop
                           unroll_loop(start, end, body, ctx)?;
                           Ok(SolveOutcome::Solved(()))
                       }
                       Err(UnknownVariable(var)) => {
                           // Range depends on unknown variable - defer
                           Ok(SolveOutcome::Deferred {
                               dependencies: vec![var],
                               description: format!(
                                   "for-loop range depends on unknown variable '{}'",
                                   var
                               ),
                           })
                       }
                   }
               }
               // ... other statement kinds
           }
       }
   }
   ```

2. **Implement range bound evaluation**
   ```rust
   /// Try to evaluate range bounds using current solution
   fn evaluate_range_bounds<'src, 'arena>(
       range: &Range<'src, 'arena>,
       ctx: &SolverContext<'src, 'arena>,
   ) -> Result<(i64, i64), EvalError> {
       let start = evaluate_const_expr(&range.start, ctx)?;
       let end = evaluate_const_expr(&range.end, ctx)?;
       Ok((start, end))
   }

   /// Evaluate expression to constant using current solution
   fn evaluate_const_expr<'src, 'arena>(
       expr: &ResolvedExpr<'src, 'arena>,
       ctx: &SolverContext<'src, 'arena>,
   ) -> Result<i64, EvalError> {
       match &expr.kind {
           ResolvedExprKind::IntLit { value } => Ok(*value),
           ResolvedExprKind::Var { name, .. } => {
               match ctx.get_variable_value(name) {
                   Some(Value::Int(v)) => Ok(*v),
                   Some(_) => Err(EvalError::TypeError),
                   None => Err(EvalError::UnknownVariable(name)),
               }
           }
           ResolvedExprKind::Add { lhs, rhs } => {
               Ok(evaluate_const_expr(lhs, ctx)? + evaluate_const_expr(rhs, ctx)?)
           }
           // ... other operators
           _ => Err(EvalError::NotConstant),
       }
   }
   ```

3. **Handle deferred constraints in solve loop**
   ```rust
   // After first solve
   if !ctx.deferred_constraints.is_empty() {
       let ready = ctx.ready_deferred_constraints();

       if ready.is_empty() {
           // No progress possible
           return Ok(SolveResult::Partial {
               solution: ctx.current_solution.unwrap(),
               deferred: ctx.deferred_constraints.clone(),
               reason: PartialReason::NoProgress {
                   stuck_constraints: ctx.deferred_constraints
                       .iter()
                       .map(|dc| dc.description.clone())
                       .collect(),
               },
               iterations: iteration + 1,
           });
       }

       // Process ready deferred constraints
       for dc in ready {
           dc.stmt.solve(ctx)?;  // Should succeed now
       }

       // Remove processed constraints
       ctx.deferred_constraints.retain(|dc| {
           !dc.dependencies.iter().all(|dep| ctx.is_variable_known(dep))
       });
   }
   ```

### Phase 3c: Testing & Refinement

1. **Write comprehensive tests**
   - For-loop with known range (immediate)
   - For-loop with unknown range (deferred then solved)
   - For-loop with unsolvable range (partial result)
   - Nested dependencies (A depends on B, B depends on C)
   - Multiple deferred constraints

2. **Error handling improvements**
   - Clear error messages for partial solutions
   - Diagnostic information about what's missing
   - Suggestions for user

3. **Performance optimization**
   - Use Z3 incremental solving (push/pop)
   - Avoid re-solving already-satisfied constraints

## Alternative Names

Instead of "partial solve", consider:

- **`solve_incremental()`** - Emphasizes the iterative nature
- **`solve_with_deferrals()`** - Explicit about deferral mechanism
- **`solve_progressive()`** - Progressive refinement of solution
- **`solve_cascading()`** - Cascading constraint resolution

**Recommendation**: Use **`solve_incremental()`** as the main API name, with `SolveResult::Partial` for partial results.

## Migration from Legacy Solver

The legacy solver can continue to exist alongside the new incremental solver:

```rust
// Legacy: fail-fast on unknown dependencies
pub fn solve<'src, 'arena>(
    statements: &[&'arena ResolvedStmt<'src, 'arena>],
    arena: &'arena Bump,
) -> Result<String, SolverError>

// New: incremental solving with deferrals
pub fn solve_incremental<'src, 'arena>(
    statements: &[&'arena ResolvedStmt<'src, 'arena>],
    ctx: &mut SolverContext<'src, 'arena>,
    config: SolveConfig,
) -> Result<SolveResult<'src, 'arena>, SolverError>

pub struct SolveConfig {
    pub max_iterations: usize,  // Default: 10
    pub allow_partial: bool,    // Default: true
    pub use_incremental_z3: bool, // Default: true
}
```

## Example Usage

```rust
// Setup
let arena = Bump::new();
let mut ctx = SolverContext::new(&arena);
let config = SolveConfig::default();

// Solve
let result = solve_incremental(&statements, &mut ctx, config)?;

match result {
    SolveResult::Complete { solution, iterations } => {
        println!("Solved in {} iterations:", iterations);
        for (var, value) in &solution.assignments {
            println!("  {} = {:?}", var, value);
        }
    }
    SolveResult::Partial { solution, deferred, reason, iterations } => {
        println!("Partial solution after {} iterations:", iterations);
        for (var, value) in &solution.assignments {
            println!("  {} = {:?}", var, value);
        }
        println!("\nCould not resolve:");
        for dc in &deferred {
            println!("  - {} (needs: {:?})", dc.description, dc.dependencies);
        }
        println!("\nReason: {:?}", reason);
    }
}
```

## Summary

This design provides:

- ✅ **Graceful handling of unknown dependencies** - defer instead of fail
- ✅ **Iterative refinement** - solve what's possible, then revisit deferred constraints
- ✅ **Clear API** - `SolveResult::Complete` vs `SolveResult::Partial`
- ✅ **Diagnostic information** - know exactly why solving was partial
- ✅ **Backward compatibility** - legacy solver can coexist
- ✅ **Extensibility** - easy to add new deferral types (functions, array access, etc.)

The implementation can be done incrementally:
1. Phase 3a: Basic iteration (1-2 days)
2. Phase 3b: For-loop deferral (2-3 days)
3. Phase 3c: Testing & refinement (1-2 days)

**Total estimate: 4-7 days**
