use crate::ast::expr::*;

// ============================================================================
// Display Implementations
// ============================================================================

impl<'src> std::fmt::Display for Expr<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::And { lhs, rhs, .. } => write!(f, "({} and {})", lhs, rhs),
            Expr::Or { lhs, rhs, .. } => write!(f, "({} or {})", lhs, rhs),
            Expr::Eq { lhs, rhs, .. } => write!(f, "({} == {})", lhs, rhs),
            Expr::NotEq { lhs, rhs, .. } => write!(f, "({} != {})", lhs, rhs),
            Expr::Add { lhs, rhs, .. } => write!(f, "({} + {})", lhs, rhs),
            Expr::Sub { lhs, rhs, .. } => write!(f, "({} - {})", lhs, rhs),
            Expr::Paren { inner, .. } => write!(f, "({})", inner),
            Expr::Mul { lhs, rhs, .. } => write!(f, "({} * {})", lhs, rhs),
            Expr::Div { lhs, rhs, .. } => write!(f, "({} / {})", lhs, rhs),
            Expr::Mod { lhs, rhs, .. } => write!(f, "({} % {})", lhs, rhs),
            Expr::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            Expr::Neg { inner, .. } => write!(f, "(-{})", inner),
            Expr::Ref { inner, .. } => write!(f, "(&{})", inner),
            Expr::Var { name, .. } => write!(f, "{}", name),
            Expr::IntLit { value, .. } => write!(f, "{}", value),
            Expr::FloatLit { value, .. } => write!(f, "{}", value),
            Expr::BoolLit { value, .. } => write!(f, "{}", value),
            Expr::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Expr::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                write!(f, "{}.{}(", receiver, method)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Expr::FieldAccess {
                receiver, field, ..
            } => write!(f, "{}.{}", receiver, field),
        }
    }
}

impl<'src> std::fmt::Display for CmpLhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmpLhs::And { lhs, rhs, .. } => write!(f, "({} and {})", lhs, rhs),
            CmpLhs::Or { lhs, rhs, .. } => write!(f, "({} or {})", lhs, rhs),
            CmpLhs::Eq { lhs, rhs, .. } => write!(f, "({} == {})", lhs, rhs),
            CmpLhs::NotEq { lhs, rhs, .. } => write!(f, "({} != {})", lhs, rhs),
            CmpLhs::Add { lhs, rhs, .. } => write!(f, "({} + {})", lhs, rhs),
            CmpLhs::Sub { lhs, rhs, .. } => write!(f, "({} - {})", lhs, rhs),
            CmpLhs::Paren { inner, .. } => write!(f, "({})", inner),
            CmpLhs::Mul { lhs, rhs, .. } => write!(f, "({} * {})", lhs, rhs),
            CmpLhs::Div { lhs, rhs, .. } => write!(f, "({} / {})", lhs, rhs),
            CmpLhs::Mod { lhs, rhs, .. } => write!(f, "({} % {})", lhs, rhs),
            CmpLhs::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            CmpLhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            CmpLhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            CmpLhs::Var { name, .. } => write!(f, "{}", name),
            CmpLhs::IntLit { value, .. } => write!(f, "{}", value),
            CmpLhs::FloatLit { value, .. } => write!(f, "{}", value),
            CmpLhs::BoolLit { value, .. } => write!(f, "{}", value),
            CmpLhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            CmpLhs::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                write!(f, "{}.{}(", receiver, method)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            CmpLhs::FieldAccess {
                receiver, field, ..
            } => write!(f, "{}.{}", receiver, field),
        }
    }
}

impl<'src> std::fmt::Display for CmpRhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmpRhs::Add { lhs, rhs, .. } => write!(f, "({} + {})", lhs, rhs),
            CmpRhs::Sub { lhs, rhs, .. } => write!(f, "({} - {})", lhs, rhs),
            CmpRhs::Paren { inner, .. } => write!(f, "({})", inner),
            CmpRhs::Mul { lhs, rhs, .. } => write!(f, "({} * {})", lhs, rhs),
            CmpRhs::Div { lhs, rhs, .. } => write!(f, "({} / {})", lhs, rhs),
            CmpRhs::Mod { lhs, rhs, .. } => write!(f, "({} % {})", lhs, rhs),
            CmpRhs::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            CmpRhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            CmpRhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            CmpRhs::Var { name, .. } => write!(f, "{}", name),
            CmpRhs::IntLit { value, .. } => write!(f, "{}", value),
            CmpRhs::FloatLit { value, .. } => write!(f, "{}", value),
            CmpRhs::BoolLit { value, .. } => write!(f, "{}", value),
            CmpRhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            CmpRhs::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                write!(f, "{}.{}(", receiver, method)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            CmpRhs::FieldAccess {
                receiver, field, ..
            } => write!(f, "{}.{}", receiver, field),
        }
    }
}

impl<'src> std::fmt::Display for AddLhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddLhs::Add { lhs, rhs, .. } => write!(f, "({} + {})", lhs, rhs),
            AddLhs::Sub { lhs, rhs, .. } => write!(f, "({} - {})", lhs, rhs),
            AddLhs::Paren { inner, .. } => write!(f, "({})", inner),
            AddLhs::Mul { lhs, rhs, .. } => write!(f, "({} * {})", lhs, rhs),
            AddLhs::Div { lhs, rhs, .. } => write!(f, "({} / {})", lhs, rhs),
            AddLhs::Mod { lhs, rhs, .. } => write!(f, "({} % {})", lhs, rhs),
            AddLhs::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            AddLhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            AddLhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            AddLhs::Var { name, .. } => write!(f, "{}", name),
            AddLhs::IntLit { value, .. } => write!(f, "{}", value),
            AddLhs::FloatLit { value, .. } => write!(f, "{}", value),
            AddLhs::BoolLit { value, .. } => write!(f, "{}", value),
            AddLhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            AddLhs::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                write!(f, "{}.{}(", receiver, method)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            AddLhs::FieldAccess {
                receiver, field, ..
            } => write!(f, "{}.{}", receiver, field),
        }
    }
}

impl<'src> std::fmt::Display for AddRhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddRhs::Paren { inner, .. } => write!(f, "({})", inner),
            AddRhs::Mul { lhs, rhs, .. } => write!(f, "({} * {})", lhs, rhs),
            AddRhs::Div { lhs, rhs, .. } => write!(f, "({} / {})", lhs, rhs),
            AddRhs::Mod { lhs, rhs, .. } => write!(f, "({} % {})", lhs, rhs),
            AddRhs::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            AddRhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            AddRhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            AddRhs::Var { name, .. } => write!(f, "{}", name),
            AddRhs::IntLit { value, .. } => write!(f, "{}", value),
            AddRhs::FloatLit { value, .. } => write!(f, "{}", value),
            AddRhs::BoolLit { value, .. } => write!(f, "{}", value),
            AddRhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            AddRhs::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                write!(f, "{}.{}(", receiver, method)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            AddRhs::FieldAccess {
                receiver, field, ..
            } => write!(f, "{}.{}", receiver, field),
        }
    }
}

impl<'src> std::fmt::Display for MulLhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MulLhs::Paren { inner, .. } => write!(f, "({})", inner),
            MulLhs::Mul { lhs, rhs, .. } => write!(f, "({} * {})", lhs, rhs),
            MulLhs::Div { lhs, rhs, .. } => write!(f, "({} / {})", lhs, rhs),
            MulLhs::Mod { lhs, rhs, .. } => write!(f, "({} % {})", lhs, rhs),
            MulLhs::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            MulLhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            MulLhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            MulLhs::Var { name, .. } => write!(f, "{}", name),
            MulLhs::IntLit { value, .. } => write!(f, "{}", value),
            MulLhs::FloatLit { value, .. } => write!(f, "{}", value),
            MulLhs::BoolLit { value, .. } => write!(f, "{}", value),
            MulLhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            MulLhs::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                write!(f, "{}.{}(", receiver, method)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            MulLhs::FieldAccess {
                receiver, field, ..
            } => write!(f, "{}.{}", receiver, field),
        }
    }
}

impl<'src> std::fmt::Display for MulRhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MulRhs::Paren { inner, .. } => write!(f, "({})", inner),
            MulRhs::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            MulRhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            MulRhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            MulRhs::Var { name, .. } => write!(f, "{}", name),
            MulRhs::IntLit { value, .. } => write!(f, "{}", value),
            MulRhs::FloatLit { value, .. } => write!(f, "{}", value),
            MulRhs::BoolLit { value, .. } => write!(f, "{}", value),
            MulRhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            MulRhs::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                write!(f, "{}.{}(", receiver, method)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            MulRhs::FieldAccess {
                receiver, field, ..
            } => write!(f, "{}.{}", receiver, field),
        }
    }
}

impl<'src> std::fmt::Display for PowLhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowLhs::Paren { inner, .. } => write!(f, "({})", inner),
            PowLhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            PowLhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            PowLhs::Var { name, .. } => write!(f, "{}", name),
            PowLhs::IntLit { value, .. } => write!(f, "{}", value),
            PowLhs::FloatLit { value, .. } => write!(f, "{}", value),
            PowLhs::BoolLit { value, .. } => write!(f, "{}", value),
            PowLhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            PowLhs::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                write!(f, "{}.{}(", receiver, method)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            PowLhs::FieldAccess {
                receiver, field, ..
            } => write!(f, "{}.{}", receiver, field),
        }
    }
}

impl<'src> std::fmt::Display for PowRhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowRhs::Paren { inner, .. } => write!(f, "({})", inner),
            PowRhs::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            PowRhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            PowRhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            PowRhs::Var { name, .. } => write!(f, "{}", name),
            PowRhs::IntLit { value, .. } => write!(f, "{}", value),
            PowRhs::FloatLit { value, .. } => write!(f, "{}", value),
            PowRhs::BoolLit { value, .. } => write!(f, "{}", value),
            PowRhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            PowRhs::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                write!(f, "{}.{}(", receiver, method)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            PowRhs::FieldAccess {
                receiver, field, ..
            } => write!(f, "{}.{}", receiver, field),
        }
    }
}

impl<'src> std::fmt::Display for Atom<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Atom::Var { name, .. } => write!(f, "{}", name),
            Atom::IntLit { value, .. } => write!(f, "{}", value),
            Atom::FloatLit { value, .. } => write!(f, "{}", value),
            Atom::BoolLit { value, .. } => write!(f, "{}", value),
            Atom::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Atom::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                write!(f, "{}.{}(", receiver, method)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Atom::FieldAccess {
                receiver, field, ..
            } => write!(f, "{}.{}", receiver, field),
        }
    }
}
