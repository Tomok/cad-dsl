use crate::ast::expr::*;

// ============================================================================
// From Implementations for Parser Convenience
// ============================================================================

/// Convert AddLhs to CmpRhs (AddLhs is a subset of CmpRhs)
impl<'src> From<AddLhs<'src>> for CmpRhs<'src> {
    fn from(add: AddLhs<'src>) -> Self {
        match add {
            AddLhs::Add { lhs, rhs, span } => CmpRhs::Add { lhs, rhs, span },
            AddLhs::Sub { lhs, rhs, span } => CmpRhs::Sub { lhs, rhs, span },
            AddLhs::Paren { inner, span } => CmpRhs::Paren { inner, span },
            AddLhs::Mul { lhs, rhs, span } => CmpRhs::Mul { lhs, rhs, span },
            AddLhs::Div { lhs, rhs, span } => CmpRhs::Div { lhs, rhs, span },
            AddLhs::Mod { lhs, rhs, span } => CmpRhs::Mod { lhs, rhs, span },
            AddLhs::Pow { lhs, rhs, span } => CmpRhs::Pow { lhs, rhs, span },
            AddLhs::Neg { inner, span } => CmpRhs::Neg { inner, span },
            AddLhs::Ref { inner, span } => CmpRhs::Ref { inner, span },
            AddLhs::Var { name, span } => CmpRhs::Var { name, span },
            AddLhs::IntLit { value, span } => CmpRhs::IntLit { value, span },
            AddLhs::FloatLit { value, span } => CmpRhs::FloatLit { value, span },
            AddLhs::BoolLit { value, span } => CmpRhs::BoolLit { value, span },
            AddLhs::Call { name, args, span } => CmpRhs::Call { name, args, span },
            AddLhs::MethodCall {
                receiver,
                method,
                args,
                span,
            } => CmpRhs::MethodCall {
                receiver,
                method,
                args,
                span,
            },
            AddLhs::FieldAccess {
                receiver,
                field,
                span,
            } => CmpRhs::FieldAccess {
                receiver,
                field,
                span,
            },
            AddLhs::ArrayLit { elements, span } => CmpRhs::ArrayLit { elements, span },
            AddLhs::StructLit { name, fields, span } => CmpRhs::StructLit { name, fields, span },
        }
    }
}

/// Convert AddLhs to CmpLhs (AddLhs is a subset of CmpLhs)
impl<'src> From<AddLhs<'src>> for CmpLhs<'src> {
    fn from(add: AddLhs<'src>) -> Self {
        match add {
            AddLhs::Add { lhs, rhs, span } => CmpLhs::Add { lhs, rhs, span },
            AddLhs::Sub { lhs, rhs, span } => CmpLhs::Sub { lhs, rhs, span },
            AddLhs::Paren { inner, span } => CmpLhs::Paren { inner, span },
            AddLhs::Mul { lhs, rhs, span } => CmpLhs::Mul { lhs, rhs, span },
            AddLhs::Div { lhs, rhs, span } => CmpLhs::Div { lhs, rhs, span },
            AddLhs::Mod { lhs, rhs, span } => CmpLhs::Mod { lhs, rhs, span },
            AddLhs::Pow { lhs, rhs, span } => CmpLhs::Pow { lhs, rhs, span },
            AddLhs::Neg { inner, span } => CmpLhs::Neg { inner, span },
            AddLhs::Ref { inner, span } => CmpLhs::Ref { inner, span },
            AddLhs::Var { name, span } => CmpLhs::Var { name, span },
            AddLhs::IntLit { value, span } => CmpLhs::IntLit { value, span },
            AddLhs::FloatLit { value, span } => CmpLhs::FloatLit { value, span },
            AddLhs::BoolLit { value, span } => CmpLhs::BoolLit { value, span },
            AddLhs::Call { name, args, span } => CmpLhs::Call { name, args, span },
            AddLhs::MethodCall {
                receiver,
                method,
                args,
                span,
            } => CmpLhs::MethodCall {
                receiver,
                method,
                args,
                span,
            },
            AddLhs::FieldAccess {
                receiver,
                field,
                span,
            } => CmpLhs::FieldAccess {
                receiver,
                field,
                span,
            },
            AddLhs::ArrayLit { elements, span } => CmpLhs::ArrayLit { elements, span },
            AddLhs::StructLit { name, fields, span } => CmpLhs::StructLit { name, fields, span },
        }
    }
}

/// Convert Atom to MulRhs (Atom is a subset of MulRhs)
impl<'src> From<Atom<'src>> for MulRhs<'src> {
    fn from(atom: Atom<'src>) -> Self {
        match atom {
            Atom::Var { name, span } => MulRhs::Var { name, span },
            Atom::IntLit { value, span } => MulRhs::IntLit { value, span },
            Atom::FloatLit { value, span } => MulRhs::FloatLit { value, span },
            Atom::BoolLit { value, span } => MulRhs::BoolLit { value, span },
            Atom::Call { name, args, span } => MulRhs::Call { name, args, span },
            Atom::MethodCall {
                receiver,
                method,
                args,
                span,
            } => MulRhs::MethodCall {
                receiver,
                method,
                args,
                span,
            },
            Atom::FieldAccess {
                receiver,
                field,
                span,
            } => MulRhs::FieldAccess {
                receiver,
                field,
                span,
            },
            Atom::ArrayLit { elements, span } => MulRhs::ArrayLit { elements, span },
            Atom::StructLit { name, fields, span } => MulRhs::StructLit { name, fields, span },
        }
    }
}

/// Convert Atom to MulLhs (Atom is a subset of MulLhs)
impl<'src> From<Atom<'src>> for MulLhs<'src> {
    fn from(atom: Atom<'src>) -> Self {
        match atom {
            Atom::Var { name, span } => MulLhs::Var { name, span },
            Atom::IntLit { value, span } => MulLhs::IntLit { value, span },
            Atom::FloatLit { value, span } => MulLhs::FloatLit { value, span },
            Atom::BoolLit { value, span } => MulLhs::BoolLit { value, span },
            Atom::Call { name, args, span } => MulLhs::Call { name, args, span },
            Atom::MethodCall {
                receiver,
                method,
                args,
                span,
            } => MulLhs::MethodCall {
                receiver,
                method,
                args,
                span,
            },
            Atom::FieldAccess {
                receiver,
                field,
                span,
            } => MulLhs::FieldAccess {
                receiver,
                field,
                span,
            },
            Atom::ArrayLit { elements, span } => MulLhs::ArrayLit { elements, span },
            Atom::StructLit { name, fields, span } => MulLhs::StructLit { name, fields, span },
        }
    }
}

/// Convert MulLhs to AddRhs (MulLhs is a subset of AddRhs)
impl<'src> From<MulLhs<'src>> for AddRhs<'src> {
    fn from(mul: MulLhs<'src>) -> Self {
        match mul {
            MulLhs::Paren { inner, span } => AddRhs::Paren { inner, span },
            MulLhs::Mul { lhs, rhs, span } => AddRhs::Mul { lhs, rhs, span },
            MulLhs::Div { lhs, rhs, span } => AddRhs::Div { lhs, rhs, span },
            MulLhs::Mod { lhs, rhs, span } => AddRhs::Mod { lhs, rhs, span },
            MulLhs::Pow { lhs, rhs, span } => AddRhs::Pow { lhs, rhs, span },
            MulLhs::Neg { inner, span } => AddRhs::Neg { inner, span },
            MulLhs::Ref { inner, span } => AddRhs::Ref { inner, span },
            MulLhs::Var { name, span } => AddRhs::Var { name, span },
            MulLhs::IntLit { value, span } => AddRhs::IntLit { value, span },
            MulLhs::FloatLit { value, span } => AddRhs::FloatLit { value, span },
            MulLhs::BoolLit { value, span } => AddRhs::BoolLit { value, span },
            MulLhs::Call { name, args, span } => AddRhs::Call { name, args, span },
            MulLhs::MethodCall {
                receiver,
                method,
                args,
                span,
            } => AddRhs::MethodCall {
                receiver,
                method,
                args,
                span,
            },
            MulLhs::FieldAccess {
                receiver,
                field,
                span,
            } => AddRhs::FieldAccess {
                receiver,
                field,
                span,
            },
            MulLhs::ArrayLit { elements, span } => AddRhs::ArrayLit { elements, span },
            MulLhs::StructLit { name, fields, span } => AddRhs::StructLit { name, fields, span },
        }
    }
}

/// Convert MulLhs to AddLhs (MulLhs is a subset of AddLhs)
impl<'src> From<MulLhs<'src>> for AddLhs<'src> {
    fn from(mul: MulLhs<'src>) -> Self {
        match mul {
            MulLhs::Paren { inner, span } => AddLhs::Paren { inner, span },
            MulLhs::Mul { lhs, rhs, span } => AddLhs::Mul { lhs, rhs, span },
            MulLhs::Div { lhs, rhs, span } => AddLhs::Div { lhs, rhs, span },
            MulLhs::Mod { lhs, rhs, span } => AddLhs::Mod { lhs, rhs, span },
            MulLhs::Pow { lhs, rhs, span } => AddLhs::Pow { lhs, rhs, span },
            MulLhs::Neg { inner, span } => AddLhs::Neg { inner, span },
            MulLhs::Ref { inner, span } => AddLhs::Ref { inner, span },
            MulLhs::Var { name, span } => AddLhs::Var { name, span },
            MulLhs::IntLit { value, span } => AddLhs::IntLit { value, span },
            MulLhs::FloatLit { value, span } => AddLhs::FloatLit { value, span },
            MulLhs::BoolLit { value, span } => AddLhs::BoolLit { value, span },
            MulLhs::Call { name, args, span } => AddLhs::Call { name, args, span },
            MulLhs::MethodCall {
                receiver,
                method,
                args,
                span,
            } => AddLhs::MethodCall {
                receiver,
                method,
                args,
                span,
            },
            MulLhs::FieldAccess {
                receiver,
                field,
                span,
            } => AddLhs::FieldAccess {
                receiver,
                field,
                span,
            },
            MulLhs::ArrayLit { elements, span } => AddLhs::ArrayLit { elements, span },
            MulLhs::StructLit { name, fields, span } => AddLhs::StructLit { name, fields, span },
        }
    }
}

/// Convert Atom to PowLhs (Atom is a subset of PowLhs)
impl<'src> From<Atom<'src>> for PowLhs<'src> {
    fn from(atom: Atom<'src>) -> Self {
        match atom {
            Atom::Var { name, span } => PowLhs::Var { name, span },
            Atom::IntLit { value, span } => PowLhs::IntLit { value, span },
            Atom::FloatLit { value, span } => PowLhs::FloatLit { value, span },
            Atom::BoolLit { value, span } => PowLhs::BoolLit { value, span },
            Atom::Call { name, args, span } => PowLhs::Call { name, args, span },
            Atom::MethodCall {
                receiver,
                method,
                args,
                span,
            } => PowLhs::MethodCall {
                receiver,
                method,
                args,
                span,
            },
            Atom::FieldAccess {
                receiver,
                field,
                span,
            } => PowLhs::FieldAccess {
                receiver,
                field,
                span,
            },
            Atom::ArrayLit { elements, span } => PowLhs::ArrayLit { elements, span },
            Atom::StructLit { name, fields, span } => PowLhs::StructLit { name, fields, span },
        }
    }
}

/// Convert Atom to PowRhs (Atom is a subset of PowRhs)
impl<'src> From<Atom<'src>> for PowRhs<'src> {
    fn from(atom: Atom<'src>) -> Self {
        match atom {
            Atom::Var { name, span } => PowRhs::Var { name, span },
            Atom::IntLit { value, span } => PowRhs::IntLit { value, span },
            Atom::FloatLit { value, span } => PowRhs::FloatLit { value, span },
            Atom::BoolLit { value, span } => PowRhs::BoolLit { value, span },
            Atom::Call { name, args, span } => PowRhs::Call { name, args, span },
            Atom::MethodCall {
                receiver,
                method,
                args,
                span,
            } => PowRhs::MethodCall {
                receiver,
                method,
                args,
                span,
            },
            Atom::FieldAccess {
                receiver,
                field,
                span,
            } => PowRhs::FieldAccess {
                receiver,
                field,
                span,
            },
            Atom::ArrayLit { elements, span } => PowRhs::ArrayLit { elements, span },
            Atom::StructLit { name, fields, span } => PowRhs::StructLit { name, fields, span },
        }
    }
}

/// Convert PowLhs to PowRhs (PowLhs is a subset of PowRhs except for Pow)
impl<'src> From<PowLhs<'src>> for PowRhs<'src> {
    fn from(pow: PowLhs<'src>) -> Self {
        match pow {
            PowLhs::Paren { inner, span } => PowRhs::Paren { inner, span },
            PowLhs::Neg { inner, span } => PowRhs::Neg { inner, span },
            PowLhs::Ref { inner, span } => PowRhs::Ref { inner, span },
            PowLhs::Var { name, span } => PowRhs::Var { name, span },
            PowLhs::IntLit { value, span } => PowRhs::IntLit { value, span },
            PowLhs::FloatLit { value, span } => PowRhs::FloatLit { value, span },
            PowLhs::BoolLit { value, span } => PowRhs::BoolLit { value, span },
            PowLhs::Call { name, args, span } => PowRhs::Call { name, args, span },
            PowLhs::MethodCall {
                receiver,
                method,
                args,
                span,
            } => PowRhs::MethodCall {
                receiver,
                method,
                args,
                span,
            },
            PowLhs::FieldAccess {
                receiver,
                field,
                span,
            } => PowRhs::FieldAccess {
                receiver,
                field,
                span,
            },
            PowLhs::ArrayLit { elements, span } => PowRhs::ArrayLit { elements, span },
            PowLhs::StructLit { name, fields, span } => PowRhs::StructLit { name, fields, span },
        }
    }
}

/// Convert PowLhs to MulRhs (PowLhs is a subset of MulRhs)
impl<'src> From<PowLhs<'src>> for MulRhs<'src> {
    fn from(pow: PowLhs<'src>) -> Self {
        match pow {
            PowLhs::Paren { inner, span } => MulRhs::Paren { inner, span },
            PowLhs::Neg { inner, span } => MulRhs::Neg { inner, span },
            PowLhs::Ref { inner, span } => MulRhs::Ref { inner, span },
            PowLhs::Var { name, span } => MulRhs::Var { name, span },
            PowLhs::IntLit { value, span } => MulRhs::IntLit { value, span },
            PowLhs::FloatLit { value, span } => MulRhs::FloatLit { value, span },
            PowLhs::BoolLit { value, span } => MulRhs::BoolLit { value, span },
            PowLhs::Call { name, args, span } => MulRhs::Call { name, args, span },
            PowLhs::MethodCall {
                receiver,
                method,
                args,
                span,
            } => MulRhs::MethodCall {
                receiver,
                method,
                args,
                span,
            },
            PowLhs::FieldAccess {
                receiver,
                field,
                span,
            } => MulRhs::FieldAccess {
                receiver,
                field,
                span,
            },
            PowLhs::ArrayLit { elements, span } => MulRhs::ArrayLit { elements, span },
            PowLhs::StructLit { name, fields, span } => MulRhs::StructLit { name, fields, span },
        }
    }
}

/// Convert PowLhs to MulLhs (PowLhs is a subset of MulLhs)
impl<'src> From<PowLhs<'src>> for MulLhs<'src> {
    fn from(pow: PowLhs<'src>) -> Self {
        match pow {
            PowLhs::Paren { inner, span } => MulLhs::Paren { inner, span },
            PowLhs::Neg { inner, span } => MulLhs::Neg { inner, span },
            PowLhs::Ref { inner, span } => MulLhs::Ref { inner, span },
            PowLhs::Var { name, span } => MulLhs::Var { name, span },
            PowLhs::IntLit { value, span } => MulLhs::IntLit { value, span },
            PowLhs::FloatLit { value, span } => MulLhs::FloatLit { value, span },
            PowLhs::BoolLit { value, span } => MulLhs::BoolLit { value, span },
            PowLhs::Call { name, args, span } => MulLhs::Call { name, args, span },
            PowLhs::MethodCall {
                receiver,
                method,
                args,
                span,
            } => MulLhs::MethodCall {
                receiver,
                method,
                args,
                span,
            },
            PowLhs::FieldAccess {
                receiver,
                field,
                span,
            } => MulLhs::FieldAccess {
                receiver,
                field,
                span,
            },
            PowLhs::ArrayLit { elements, span } => MulLhs::ArrayLit { elements, span },
            PowLhs::StructLit { name, fields, span } => MulLhs::StructLit { name, fields, span },
        }
    }
}
