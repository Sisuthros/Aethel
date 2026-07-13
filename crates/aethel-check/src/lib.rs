// v0.1: basic (full HIR later)
pub mod checker {
    use aethel_syntax::ast::Module;
    use aethel_syntax::span::FileId;
    use aethel_ir::lower::IrModule;
    use aethel_syntax::diagnostic::Diagnostics;
    pub fn check_module(_module: &Module, file_id: FileId) -> (IrModule, Diagnostics) {
        (IrModule { file_id, items: vec![] }, Diagnostics::new())
    }
}
pub mod epistemic {}
pub mod types {}
