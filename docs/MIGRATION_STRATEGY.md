# Solver Architecture Migration Strategy

## Current State

The existing solver implementation (~8500 lines) uses an **imperative extraction** approach:
- Single `extract_constraints()` function walks the HIR
- Builds a `ConstraintProblem` structure
- Passes to Z3 via `z3_bridge.rs`

**Existing files:**
- `constraint_extractor.rs` (3053 lines) - Main extraction logic
- `function_inliner.rs` (1224 lines) - Function inlining
- `z3_bridge.rs` (2483 lines) - Z3 interface
- `struct_flattener.rs` (623 lines) - Struct/array flattening
- `solution_formatter.rs` (655 lines) - Output formatting
- `recursive_struct_detector.rs` (421 lines) - Cycle detection

## Target State

New **trait-based** architecture:
- HIR nodes implement `Solvable` trait
- Modular `impls/` subdirectory
- Tree-based variable management
- RAII scope guards

## Migration Strategy: Incremental Refactoring

**DO NOT delete existing code!** Instead, migrate incrementally:

### Phase 0: Preparation (1-2 hours)

1. **Create new branch**: `git checkout -b refactor/trait-based-solver`
2. **Rename existing solver**: `git mv src/solver src/solver_legacy`
3. **Update imports**: Change `use crate::solver` → `use crate::solver_legacy`
4. **Verify**: Existing code still compiles with legacy solver
5. **Commit**: "Preserve legacy solver implementation"

**Why?** This keeps the working implementation available while we build the new one.

### Phase 1: Extract Reusable Components (2-4 hours)

Create new `src/solver/` with reusable parts from legacy:

```bash
# Copy reusable modules (with modifications)
cp src/solver_legacy/struct_flattener.rs src/solver/
cp src/solver_legacy/recursive_struct_detector.rs src/solver/
cp src/solver_legacy/solution_formatter.rs src/solver/

# These can be reused almost as-is
```

**Modifications needed:**
- Update module paths
- Extract pure functions
- Remove coupling to old `ConstraintProblem` type

### Phase 2: New Core Infrastructure (4-8 hours)

Create new trait-based core in `src/solver/`:

1. **Create `src/solver.rs`**:
   - Define `Solvable` trait
   - Define core types (`VariablePath`, `PathComponent`)
   - Module declarations

2. **Create `src/solver/context.rs`**:
   - `SolverContext` with tree-based variables
   - RAII guards (`ScopeGuard`, `WithGuard`)
   - Scope management

3. **Create `src/solver/impls.rs`** + `src/solver/impls/`:
   - Basic `expr.rs` - literals, variables, binary ops
   - Basic `stmt.rs` - let statements, constraints

**Reuse from legacy:**
- Z3 variable creation patterns from `z3_bridge.rs`
- Type flattening logic from `struct_flattener.rs`

### Phase 3: Port Functionality (12-20 hours)

Incrementally port features, testing after each. Phase 3 now includes **iterative partial solving** to handle dependencies that are resolved progressively.

**✅ COMPLETED:**

**Priority 1 - Basic constraints:**
- [x] Let statements (initialized/uninitialized)
- [x] Expression statements (constraints)
- [x] Binary operations (+, -, *, /)
- [x] Comparisons (==, !=, <, >, <=, >=)

**Priority 2 - Structs:**
- [x] Struct declarations
- [x] Struct field access
- [x] Struct flattening (reuse `struct_flattener.rs`)

**Priority 3 - Arrays:**
- [x] Array types
- [x] Array indexing (constant indices)
- [x] Array flattening

**Priority 4 - Control flow:**
- [x] If statements (conditional constraints)

**Priority 5 - Container with-statements:**
- [x] Container context
- [x] Dot-prefix syntax
- [x] Namespace resolution

**🚧 IN PROGRESS - Partial Solve Architecture:**

See `docs/PARTIAL_SOLVE_DESIGN.md` for detailed design.

**Phase 3a: Basic Iterative Solving (1-2 days)**
- [ ] Solution extraction from Z3 model
- [ ] Iterative solve loop with progress tracking
- [ ] `SolveResult` enum (Complete/Partial)
- [ ] Progress detection (count resolved variables)
- [ ] Basic tests for iteration mechanics

**Phase 3b: For-Loop Deferral - Priority 4 (2-3 days)**
- [ ] For loops with known ranges (immediate unrolling)
- [ ] For loops with unknown ranges (deferred)
- [ ] Range bound evaluation using current solution
- [ ] `DeferredConstraint` tracking
- [ ] Re-evaluate deferred loops after progress
- [ ] Tests for for-loop deferral scenarios

**Phase 3c: Function Deferral - Priority 6 (2-3 days)**
- [ ] Function inlining (adapt `function_inliner.rs`)
- [ ] Method calls
- [ ] Parameter binding with current solution
- [ ] Defer function calls with unknown parameters
- [ ] Tests for function deferral scenarios

**Phase 3d: Testing & Refinement (1-2 days)**
- [ ] Comprehensive integration tests
- [ ] Error message improvements
- [ ] Performance profiling
- [ ] Documentation updates

**Priority 7 - Transforms (Future):**
- [ ] Transform with-statements
- [ ] Shadow variables
- [ ] Auto-call __transform__

**Key Changes from Original Plan:**
- Phase 3 now subdivided into 3a, 3b, 3c, 3d
- Iterative solving with deferral mechanism for Priorities 4 & 6
- Progress-based iteration (continues while making progress)
- Both Complete and Partial results are valid outcomes

### Phase 4: Integration & Testing (4-8 hours)

1. **Update `src/lib.rs` or `src/main.rs`**:
   ```rust
   // Switch from legacy to new solver
   // use crate::solver_legacy as solver;
   use crate::solver;
   ```

2. **Run existing integration tests**
3. **Add new tests for trait-based features**
4. **Performance comparison**: legacy vs new

### Phase 5: Cleanup (2-4 hours)

Once new solver is working:

1. **Delete legacy**: `rm -rf src/solver_legacy/`
2. **Update documentation**
3. **Final testing**
4. **Merge to main**

## What to Reuse vs Rewrite

### Reuse Directly (minimal changes):
- ✅ `struct_flattener.rs` - Pure logic, just update types
- ✅ `recursive_struct_detector.rs` - Independent utility
- ✅ `solution_formatter.rs` - Output formatting

### Adapt & Port (moderate changes):
- 🔄 `z3_bridge.rs` - Extract Z3 conversion patterns
  - Reuse: Type→Z3 mapping logic
  - Rewrite: Integration with new context
- 🔄 `function_inliner.rs` - Core inlining logic
  - Reuse: Parameter substitution
  - Rewrite: Integrate with `Solvable` trait

### Rewrite from Scratch:
- ❌ `constraint_extractor.rs` - Replaced by trait implementations
  - Old: Monolithic function
  - New: Distributed `Solvable` impls

## Benefits of This Approach

1. **Safety**: Working implementation always available
2. **Incremental**: Can test after each step
3. **Learning**: Understand existing code before replacing
4. **Reuse**: Don't throw away good code (struct flattening, Z3 interface)
5. **Comparison**: Can compare old vs new performance

## Time Estimate

- **Phase 0**: 1-2 hours (setup) ✅ DONE
- **Phase 1**: 2-4 hours (extract reusable) ✅ DONE
- **Phase 2**: 4-8 hours (new core) ✅ DONE
- **Phase 3**: 12-20 hours (port features + partial solve) 🚧 IN PROGRESS
  - Phase 3a: 1-2 days (basic iterative solving)
  - Phase 3b: 2-3 days (for-loop deferral)
  - Phase 3c: 2-3 days (function deferral)
  - Phase 3d: 1-2 days (testing & refinement)
- **Phase 4**: 4-8 hours (integration)
- **Phase 5**: 2-4 hours (cleanup)

**Total: 29-50 hours** (4-7 working days)

## Risk Mitigation

- ✅ Use git branches
- ✅ Keep legacy code during migration
- ✅ Test incrementally
- ✅ Compare behavior with existing tests
- ✅ Document what's ported vs new

## Next Steps

1. Review this strategy with team
2. Create migration branch
3. Start with Phase 0
4. Update this document as you learn more
