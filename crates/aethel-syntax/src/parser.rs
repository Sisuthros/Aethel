//! Handwritten recursive-descent parser with Pratt parser for expressions.

use crate::ast::*;
use crate::diagnostic::{codes, DiagnosticBuilder, DiagnosticSeverity, Diagnostics};
use crate::lexer::{Token, TokenKind, EMPTY_TOKEN};
use crate::span::{ByteOffset, FileId, Span, Spanned};
use indexmap::IndexMap;
use smallvec::SmallVec;
use std::fmt;

/// Parser for Aethel source code.
pub struct Parser<'a> {
    tokens: &'a [Token],
    current: usize,
    file_id: FileId,
    diagnostics: Diagnostics,
}

/// Precedence levels for Pratt parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Lowest = 0,
    Assignment = 1, // =
    Or = 2,         // ||
    And = 3,        // &&
    Equality = 4,   // == !=
    Comparison = 5, // < > <= >=
    Term = 6,       // + -
    Factor = 7,     // * / %
    Unary = 8,      // ! - *
    Call = 9,       // () [] .
    Primary = 10,
}

impl Precedence {
    fn next(self) -> Self {
        match self {
            Precedence::Lowest => Precedence::Assignment,
            Precedence::Assignment => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => Precedence::Primary,
        }
    }
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token], file_id: FileId) -> Self {
        Self {
            tokens,
            current: 0,
            file_id,
            diagnostics: Diagnostics::new(),
        }
    }

    pub fn parse(mut self) -> (Module, Diagnostics) {
        let module = self.parse_module();
        (module, self.diagnostics)
    }

    fn parse_module(&mut self) -> Module {
        let start_span = self.current_span();
        let mut items = Vec::new();

        while !self.is_at_end() {
            if let Some(item) = self.parse_item() {
                items.push(item);
            } else {
                // Error recovery: skip to next item
                self.skip_to_next_item();
            }
        }

        let end_span = if self.current > 0 {
            self.previous_span()
        } else {
            start_span
        };
        let span = start_span.merge(end_span);
        Module { span, items }
    }

    fn parse_item(&mut self) -> Option<Item> {
        let start = self.current_span();
        let is_pub = self.eat(TokenKind::KwPub);

        // Check for use declarations
        if self.check(TokenKind::KwUse) {
            return Some(Item::Use(self.parse_use_decl(is_pub, start)?));
        }

        // Check for module declarations
        if self.check(TokenKind::KwMod) {
            return Some(Item::Mod(self.parse_mod_decl(is_pub, start)?));
        }

        // Check for type aliases
        if self.check(TokenKind::KwType) {
            return Some(Item::TypeAlias(self.parse_type_alias(is_pub, start)?));
        }

        // Check for struct
        if self.check(TokenKind::KwStruct) {
            return Some(Item::Struct(self.parse_struct_def(is_pub, start)?));
        }

        // Check for enum
        if self.check(TokenKind::KwEnum) {
            return Some(Item::Enum(self.parse_enum_def(is_pub, start)?));
        }

        // Check for policy
        if self.check(TokenKind::KwPolicy) || self.check(TokenKind::KwClaim) {
            return Some(Item::Policy(self.parse_policy_def(is_pub, start)?));
        }

        // Check for effect
        if self.check(TokenKind::KwEffect) {
            return Some(Item::Effect(self.parse_effect_def(is_pub, start)?));
        }

        // Check for function
        if self.check(TokenKind::KwFn) {
            return Some(Item::Fn(self.parse_fn_def(is_pub, start)?));
        }

        // Unexpected token
        self.error(
            codes::PARSE_ERROR(),
            "expected item (fn, struct, enum, type, use, mod, policy)",
        );
        None
    }

    fn parse_use_decl(&mut self, is_pub: bool, start: Span) -> Option<UseDecl> {
        self.eat(TokenKind::KwUse);
        let path = self.parse_use_path()?;
        self.expect(TokenKind::Semi, "expected `;` after use declaration")?;
        let end = self.previous_span();
        Some(UseDecl {
            span: start.merge(end),
            path,
            is_pub,
        })
    }

    fn parse_use_path(&mut self) -> Option<UsePath> {
        let start = self.current_span();
        let prefix = self.parse_type_path()?;

        if self.eat(TokenKind::DotDotDot) {
            return Some(UsePath::Glob {
                span: start.merge(self.previous_span()),
                prefix,
            });
        }

        if self.eat(TokenKind::LBrace) {
            let mut items = Vec::new();
            while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                items.push(self.parse_use_path()?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RBrace, "expected `}`")?;
            let end = self.previous_span();
            return Some(UsePath::Group {
                span: start.merge(end),
                prefix,
                items,
            });
        }

        Some(UsePath::Simple {
            span: prefix.span,
            path: prefix,
        })
    }

    fn parse_mod_decl(&mut self, is_pub: bool, start: Span) -> Option<ModDecl> {
        self.eat(TokenKind::KwMod);
        let name = self.parse_ident()?;

        let body = if self.eat(TokenKind::LBrace) {
            let mut items = Vec::new();
            while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                if let Some(item) = self.parse_item() {
                    items.push(item);
                } else {
                    self.skip_to_next_item();
                }
            }
            self.expect(TokenKind::RBrace, "expected `}`")?;
            let end = self.previous_span();
            let module_span = start.merge(end);
            Some(Module {
                span: module_span,
                items,
            })
        } else {
            self.expect(TokenKind::Semi, "expected `;` or `{` after module name")?;
            None
        };

        let end = self.previous_span();
        Some(ModDecl {
            span: start.merge(end),
            name,
            body,
            is_pub,
        })
    }

    fn parse_type_alias(&mut self, is_pub: bool, start: Span) -> Option<TypeAlias> {
        self.eat(TokenKind::KwType);
        let name = self.parse_ident()?;

        let generics = if self.eat(TokenKind::Lt) {
            let mut generics = Vec::new();
            while !self.check(TokenKind::Gt) && !self.is_at_end() {
                generics.push(self.parse_generic_param()?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Gt, "expected `>`")?;
            generics
        } else {
            Vec::new()
        };

        self.expect(TokenKind::Eq, "expected `=` in type alias")?;
        let ty = self.parse_type()?;
        self.expect(TokenKind::Semi, "expected `;` after type alias")?;

        let end = self.previous_span();
        Some(TypeAlias {
            span: start.merge(end),
            name,
            generics,
            ty,
            is_pub,
        })
    }

    fn parse_struct_def(&mut self, is_pub: bool, start: Span) -> Option<StructDef> {
        self.eat(TokenKind::KwStruct);
        let name = self.parse_ident()?;

        let generics = if self.eat(TokenKind::Lt) {
            let mut generics = Vec::new();
            while !self.check(TokenKind::Gt) && !self.is_at_end() {
                generics.push(self.parse_generic_param()?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Gt, "expected `>`")?;
            generics
        } else {
            Vec::new()
        };

        self.expect(TokenKind::LBrace, "expected `{` after struct name")?;
        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let field_start = self.current_span();
            let is_pub = self.eat(TokenKind::KwPub);
            let field_name = self.parse_ident()?;
            self.expect(TokenKind::Colon, "expected `:` after field name")?;
            let ty = self.parse_type()?;
            self.eat(TokenKind::Comma);
            let field_end = self.previous_span();
            fields.push(StructField {
                span: field_start.merge(field_end),
                name: field_name,
                ty,
                is_pub,
            });
        }
        self.expect(TokenKind::RBrace, "expected `}`")?;

        let end = self.previous_span();
        Some(StructDef {
            span: start.merge(end),
            name,
            generics,
            fields,
            is_pub,
        })
    }

    fn parse_enum_def(&mut self, is_pub: bool, start: Span) -> Option<EnumDef> {
        self.eat(TokenKind::KwEnum);
        let name = self.parse_ident()?;

        let generics = if self.eat(TokenKind::Lt) {
            let mut generics = Vec::new();
            while !self.check(TokenKind::Gt) && !self.is_at_end() {
                generics.push(self.parse_generic_param()?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Gt, "expected `>`")?;
            generics
        } else {
            Vec::new()
        };

        self.expect(TokenKind::LBrace, "expected `{` after enum name")?;
        let mut variants = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let var_start = self.current_span();
            let name = self.parse_ident()?;

            let fields = if self.eat(TokenKind::LParen) {
                let mut fields = Vec::new();
                while !self.check(TokenKind::RParen) && !self.is_at_end() {
                    let field_start = self.current_span();
                    let ty = self.parse_type()?;
                    let field_end = self.previous_span();
                    fields.push(EnumField::Tuple {
                        span: field_start.merge(field_end),
                        ty,
                    });
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::RParen, "expected `)`")?;
                fields
            } else if self.eat(TokenKind::LBrace) {
                let mut fields = Vec::new();
                while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                    let field_start = self.current_span();
                    let name = self.parse_ident()?;
                    self.expect(TokenKind::Colon, "expected `:` after field name")?;
                    let ty = self.parse_type()?;
                    self.eat(TokenKind::Comma);
                    let field_end = self.previous_span();
                    fields.push(EnumField::Named {
                        span: field_start.merge(field_end),
                        name,
                        ty,
                    });
                }
                self.expect(TokenKind::RBrace, "expected `}`")?;
                fields
            } else {
                Vec::new()
            };

            self.eat(TokenKind::Comma);
            let var_end = self.previous_span();
            variants.push(EnumVariant {
                span: var_start.merge(var_end),
                name,
                fields,
            });
        }
        self.expect(TokenKind::RBrace, "expected `}`")?;

        let end = self.previous_span();
        Some(EnumDef {
            span: start.merge(end),
            name,
            generics,
            variants,
            is_pub,
        })
    }

    fn parse_policy_def(&mut self, is_pub: bool, start: Span) -> Option<PolicyDef> {
        let is_claim = self.eat(TokenKind::KwClaim);

        if is_claim {
            // Handle `Claim<T>` as a type, not a policy definition
            // `claim` (lowercase) could start a standalone policy beginning with `claim`
            self.error(
                codes::PARSE_ERROR(),
                "`Claim` is a type, not a standalone definition",
            );
            return None;
        }

        self.eat(TokenKind::KwPolicy);
        let name = self.parse_ident()?;

        let generics = if self.eat(TokenKind::Lt) {
            let mut generics = Vec::new();
            while !self.check(TokenKind::Gt) && !self.is_at_end() {
                generics.push(self.parse_generic_param()?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Gt, "expected `>`")?;
            generics
        } else {
            Vec::new()
        };

        self.expect(TokenKind::LBrace, "expected `{` after policy name")?;
        let mut claims = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let claim_start = self.current_span();
            let name = self.parse_ident()?;
            self.expect(TokenKind::Colon, "expected `:` after claim name")?;
            let ty = self.parse_type()?;

            let mut evidence = Vec::new();
            if self.eat(TokenKind::LBrace) {
                while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                    let ev_start = self.current_span();
                    let kind = self.parse_evidence_kind()?;
                    let description = if let Some(s) = self.parse_string_literal() {
                        s
                    } else {
                        String::new()
                    };
                    self.eat(TokenKind::Comma);
                    let ev_end = self.previous_span();
                    evidence.push(EvidenceReq {
                        span: ev_start.merge(ev_end),
                        kind,
                        description,
                    });
                }
                self.expect(TokenKind::RBrace, "expected `}`")?;
            }

            let claim_end = self.previous_span();
            claims.push(PolicyClaim {
                span: claim_start.merge(claim_end),
                name,
                ty,
                evidence,
            });

            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RBrace, "expected `}`")?;

        let end = self.previous_span();
        Some(PolicyDef {
            span: start.merge(end),
            name,
            generics,
            claims,
            is_pub,
        })
    }

    fn parse_effect_def(&mut self, is_pub: bool, start: Span) -> Option<EffectDef> {
        self.eat(TokenKind::KwEffect);
        let name = self.parse_ident()?;

        // optional generics
        if self.eat(TokenKind::Lt) {
            while !self.check(TokenKind::Gt) && !self.is_at_end() {
                let _ = self.parse_generic_param();
                self.eat(TokenKind::Comma);
            }
            let _ = self.eat(TokenKind::Gt);
        }

        self.expect(TokenKind::LBrace, "expected `{` after effect name")?;

        // Parse effect operations (each starts with `fn`)
        let mut operations = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            // Consume whitespace/newlines between operations
            if self.eat(TokenKind::Semi) {
                continue;
            }

            // Each operation starts with `fn`
            if !self.check(TokenKind::KwFn) {
                // Skip unexpected token inside effect body
                self.advance();
                continue;
            }

            let op_start = self.current_span();
            self.eat(TokenKind::KwFn);
            let op_name = self.parse_ident()?;

            // Parameters
            self.expect(TokenKind::LParen, "expected `(` after operation name")?;
            let mut params = Vec::new();
            while !self.check(TokenKind::RParen) && !self.is_at_end() {
                let param_start = self.current_span();
                let is_mut = self.eat(TokenKind::KwMut);
                let param_name = self.parse_ident()?;
                self.expect(TokenKind::Colon, "expected `:` after parameter name")?;
                let ty = self.parse_type()?;
                self.eat(TokenKind::Comma);
                let param_end = self.previous_span();
                params.push(Param {
                    span: param_start.merge(param_end),
                    name: param_name,
                    ty,
                    is_mut,
                });
            }
            self.expect(TokenKind::RParen, "expected `)`")?;

            // Return type
            let ret_type = if self.eat(TokenKind::Arrow) {
                Some(self.parse_type()?)
            } else {
                None
            };

            let op_end = self.previous_span();
            operations.push(EffectOperation {
                span: op_start.merge(op_end),
                name: op_name,
                params,
                ret_type,
            });
        }

        self.expect(TokenKind::RBrace, "expected `}`")?;

        let end = self.previous_span();
        Some(EffectDef {
            span: start.merge(end),
            name,
            operations,
            is_pub,
        })
    }

    fn parse_evidence_kind(&mut self) -> Option<EvidenceKind> {
        // `evidence` keyword is required, then the evidence kind identifier
        if !self.eat(TokenKind::KwEvidence) {
            self.error(codes::PARSE_ERROR(), "expected `evidence` keyword");
            return None;
        }
        if let Some(ident) = self.parse_optional_ident() {
            match ident.name.as_str() {
                "SignedAttestation" => Some(EvidenceKind::SignedAttestation),
                "CryptographicProof" => Some(EvidenceKind::CryptographicProof),
                "AuditLog" => Some(EvidenceKind::AuditLog),
                "HumanReview" => Some(EvidenceKind::HumanReview),
                s => Some(EvidenceKind::Custom(s.to_string())),
            }
        } else {
            self.error(
                codes::PARSE_ERROR(),
                "expected evidence kind (SignedAttestation, CryptographicProof, AuditLog, HumanReview, or Custom)",
            );
            None
        }
    }

    fn parse_optional_ident(&mut self) -> Option<Ident> {
        if let TokenKind::Ident(name) = &self.current_token().kind {
            let span = self.current_token().span;
            let ident = Ident::new(span, name.clone());
            self.advance();
            Some(ident)
        } else {
            None
        }
    }

    fn parse_fn_def(&mut self, is_pub: bool, start: Span) -> Option<FnDef> {
        self.eat(TokenKind::KwFn);
        let name = self.parse_ident()?;

        let generics = if self.eat(TokenKind::Lt) {
            let mut generics = Vec::new();
            while !self.check(TokenKind::Gt) && !self.is_at_end() {
                generics.push(self.parse_generic_param()?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Gt, "expected `>`")?;
            generics
        } else {
            Vec::new()
        };

        self.expect(TokenKind::LParen, "expected `(` after function name")?;
        let mut params = Vec::new();
        while !self.check(TokenKind::RParen) && !self.is_at_end() {
            let param_start = self.current_span();
            let is_mut = self.eat(TokenKind::KwMut);
            let name = self.parse_ident()?;
            self.expect(TokenKind::Colon, "expected `:` after parameter name")?;
            let ty = self.parse_type()?;
            self.eat(TokenKind::Comma);
            let param_end = self.previous_span();
            params.push(Param {
                span: param_start.merge(param_end),
                name,
                ty,
                is_mut,
            });
        }
        self.expect(TokenKind::RParen, "expected `)`")?;

        let ret_type = if self.eat(TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let effects = if self.eat(TokenKind::KwUses) {
            let effect_start = self.current_span();
            let mut effects = Vec::new();
            while !self.check(TokenKind::LBrace)
                && !self.check(TokenKind::Colon)
                && !self.check(TokenKind::Semi)
                && !self.is_at_end()
            {
                effects.push(self.parse_effect_ref()?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            let effect_end = self.previous_span();
            EffectSet {
                span: effect_start.merge(effect_end),
                effects,
            }
        } else {
            EffectSet::default()
        };

        let body = {
            // Consume optional colon before function body
            if self.eat(TokenKind::Colon) {
                // colon consumed
            }
            if self.check(TokenKind::LBrace) {
                Some(self.parse_block()?)
            } else {
                self.expect(
                    TokenKind::Semi,
                    "expected `;` or `{` after function signature",
                )?;
                None
            }
        };

        let end = self.previous_span();
        Some(FnDef {
            span: start.merge(end),
            name,
            generics,
            params,
            ret_type,
            effects,
            body,
            is_pub,
        })
    }

    fn parse_generic_param(&mut self) -> Option<GenericParam> {
        let start = self.current_span();
        let name = self.parse_ident()?;

        let bounds = if self.eat(TokenKind::Colon) {
            let mut bounds = Vec::new();
            while !self.check(TokenKind::Comma) && !self.check(TokenKind::Gt) && !self.is_at_end() {
                bounds.push(self.parse_type_bound()?);
                if !self.eat(TokenKind::Plus) {
                    break;
                }
            }
            bounds
        } else {
            Vec::new()
        };

        let end = self.previous_span();
        Some(GenericParam {
            span: start.merge(end),
            name,
            bounds,
        })
    }

    fn parse_type_bound(&mut self) -> Option<TypeBound> {
        let start = self.current_span();
        let path = self.parse_type_path()?;
        let end = self.previous_span();
        Some(TypeBound {
            span: start.merge(end),
            path,
        })
    }

    fn parse_effect_ref(&mut self) -> Option<EffectRef> {
        let start = self.current_span();
        let path = self.parse_type_path()?;
        let end = self.previous_span();
        Some(EffectRef {
            span: start.merge(end),
            path,
        })
    }

    // Statement parsing
    fn parse_block(&mut self) -> Option<Block> {
        let start = self.current_span();
        // `parse_block` owns the opening brace. Every caller must leave it
        // unconsumed: the block-statement and block-expression sites only
        // `check` for `{`, so a lenient `eat` here would let the parser
        // recurse without consuming a token and overflow the stack.
        self.expect(TokenKind::LBrace, "expected `{`")?;
        let mut stmts = Vec::new();
        let mut tail = None;

        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            if self.check(TokenKind::Semi) {
                self.advance();
                continue;
            }

            // Check if this could be a tail expression (no semicolon, followed by } or another stmt)
            let is_tail = !self.check(TokenKind::Semi)
                && (self.check(TokenKind::RBrace)
                    || (self.peek(1).map_or(false, |t| {
                        matches!(
                            t.kind,
                            TokenKind::RBrace
                                | TokenKind::KwLet
                                | TokenKind::KwReturn
                                | TokenKind::KwIf
                                | TokenKind::KwWhile
                                | TokenKind::KwFor
                                | TokenKind::KwMatch
                                | TokenKind::KwAsk
                                | TokenKind::KwVerify
                                | TokenKind::KwCommit
                        )
                    })));

            if let Some(stmt) = self.parse_stmt() {
                if is_tail && matches!(stmt, Stmt::Expr { .. }) {
                    if let Stmt::Expr { expr, .. } = stmt {
                        tail = Some(Box::new(expr));
                    }
                } else {
                    stmts.push(stmt);
                }
            } else {
                self.skip_to_next_item();
            }
        }

        self.expect(TokenKind::RBrace, "expected `}`")?;
        let end = self.previous_span();

        Some(Block {
            span: start.merge(end),
            stmts,
            tail,
        })
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        let start = self.current_span();

        // let statement
        if self.eat(TokenKind::KwLet) {
            let is_mut = self.eat(TokenKind::KwMut);
            let pat = self.parse_pat()?;

            let ty = if self.eat(TokenKind::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };

            let init = if self.eat(TokenKind::Eq) {
                Some(self.parse_expr(Precedence::Lowest)?)
            } else {
                None
            };

            self.expect(TokenKind::Semi, "expected `;` after let statement")?;
            let end = self.previous_span();

            return Some(Stmt::Let {
                span: start.merge(end),
                name: match pat {
                    Pat::Ident { name, .. } => name,
                    _ => {
                        self.error(codes::PARSE_ERROR(), "expected identifier in let binding");
                        Ident::dummy("error")
                    }
                },
                ty,
                is_mut,
                init,
            });
        }

        // return statement
        if self.eat(TokenKind::KwReturn) {
            let expr = if !self.check(TokenKind::Semi) && !self.check(TokenKind::RBrace) {
                Some(self.parse_expr(Precedence::Lowest)?)
            } else {
                None
            };
            self.expect(TokenKind::Semi, "expected `;` after return")?;
            let end = self.previous_span();
            return Some(Stmt::Return {
                span: start.merge(end),
                expr,
            });
        }

        // if statement
        if self.eat(TokenKind::KwIf) {
            let cond = self.parse_expr(Precedence::Lowest)?;
            let then_branch = self.parse_block()?;

            let else_branch = if self.eat(TokenKind::KwElse) {
                if self.check(TokenKind::KwIf) {
                    Some(Box::new(self.parse_stmt()?))
                } else {
                    Some(Box::new(Stmt::Block {
                        span: self.current_span(),
                        block: self.parse_block()?,
                    }))
                }
            } else {
                None
            };

            let end = self.previous_span();
            return Some(Stmt::If {
                span: start.merge(end),
                cond,
                then_branch,
                else_branch,
            });
        }

        // while statement
        if self.eat(TokenKind::KwWhile) {
            let cond = self.parse_expr(Precedence::Lowest)?;
            let body = self.parse_block()?;
            let end = self.previous_span();
            return Some(Stmt::While {
                span: start.merge(end),
                cond,
                body,
            });
        }

        // for statement
        if self.eat(TokenKind::KwFor) {
            let pat = self.parse_pat()?;
            self.expect(TokenKind::KwIn, "expected `in`")?;
            let iter = self.parse_expr(Precedence::Lowest)?;
            let body = self.parse_block()?;
            let end = self.previous_span();
            return Some(Stmt::For {
                span: start.merge(end),
                pat,
                iter,
                body,
            });
        }

        // match statement
        if self.eat(TokenKind::KwMatch) {
            let scrutinee = self.parse_expr(Precedence::Lowest)?;
            self.expect(TokenKind::LBrace, "expected `{`")?;
            let mut arms = Vec::new();
            while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                let arm_start = self.current_span();
                let pat = self.parse_pat()?;
                let guard = if self.eat(TokenKind::KwIf) {
                    Some(self.parse_expr(Precedence::Lowest)?)
                } else {
                    None
                };
                self.expect(TokenKind::FatArrow, "expected `=>`")?;
                let body = self.parse_expr(Precedence::Lowest)?;
                self.eat(TokenKind::Comma);
                let arm_end = self.previous_span();
                arms.push(MatchArm {
                    span: arm_start.merge(arm_end),
                    pat,
                    guard,
                    body,
                });
            }
            self.expect(TokenKind::RBrace, "expected `}`")?;
            let end = self.previous_span();
            return Some(Stmt::Match {
                span: start.merge(end),
                scrutinee,
                arms,
            });
        }

        // Block statement
        if self.check(TokenKind::LBrace) {
            let block = self.parse_block()?;
            let end = self.previous_span();
            return Some(Stmt::Block {
                span: start.merge(end),
                block,
            });
        }

        // Expression statement
        let expr = self.parse_expr(Precedence::Lowest)?;
        let end = self.previous_span();

        // Check if it's a special expression that can be a statement
        match &expr {
            Expr::Ask { .. }
            | Expr::Verify { .. }
            | Expr::CommitOnce { .. }
            | Expr::Return { .. }
            | Expr::Break { .. }
            | Expr::Continue { .. }
            | Expr::Call { .. }
            | Expr::MethodCall { .. } => {
                self.eat(TokenKind::Semi);
            }
            _ => {
                // Require semicolon for other expressions
                if !self.check(TokenKind::RBrace) && !self.is_at_end() {
                    self.expect(TokenKind::Semi, "expected `;` after expression")?;
                }
            }
        }

        Some(Stmt::Expr {
            span: start.merge(end),
            expr,
        })
    }

    // Pattern parsing
    fn parse_pat(&mut self) -> Option<Pat> {
        let start = self.current_span();

        // Wildcard
        if self.eat(TokenKind::Question) {
            let end = self.previous_span();
            return Some(Pat::Wild {
                span: start.merge(end),
            });
        }

        // Reference pattern
        if self.eat(TokenKind::And) {
            let is_mut = self.eat(TokenKind::KwMut);
            let pat = self.parse_pat()?;
            let end = self.previous_span();
            return Some(Pat::Ref {
                span: start.merge(end),
                is_mut,
                pat: Box::new(pat),
            });
        }

        // Literal pattern
        if let Some(lit) = self.parse_literal() {
            let end = self.previous_span();
            return Some(Pat::Literal {
                span: start.merge(end),
                lit,
            });
        }

        // Identifier or path pattern
        if self.check(TokenKind::Ident(String::new())) {
            let ident = self.parse_ident()?;

            // Check for struct pattern
            if self.check(TokenKind::LBrace) {
                self.eat(TokenKind::LBrace);
                let mut fields = Vec::new();
                while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                    let field_start = self.current_span();
                    let name = self.parse_ident()?;
                    let pat = if self.eat(TokenKind::Colon) {
                        Some(self.parse_pat()?)
                    } else {
                        None
                    };
                    self.eat(TokenKind::Comma);
                    let field_end = self.previous_span();
                    fields.push(PatField {
                        span: field_start.merge(field_end),
                        name,
                        pat,
                    });
                }
                self.expect(TokenKind::RBrace, "expected `}`")?;
                let end = self.previous_span();
                return Some(Pat::Struct {
                    span: start.merge(end),
                    path: TypePath::single(ident.span, ident),
                    fields,
                });
            }

            // Check for enum/tuple pattern
            if self.check(TokenKind::LParen) {
                self.eat(TokenKind::LParen);
                let mut pats = Vec::new();
                while !self.check(TokenKind::RParen) && !self.is_at_end() {
                    pats.push(self.parse_pat()?);
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::RParen, "expected `)`")?;
                let end = self.previous_span();
                return Some(Pat::Tuple {
                    span: start.merge(end),
                    pats,
                });
            }

            // Check for enum variant pattern (path::Variant(args))
            if self.check(TokenKind::ColonColon) {
                let path = self.parse_expr_path_starting_with(ident.clone())?;
                if self.check(TokenKind::LParen) {
                    self.eat(TokenKind::LParen);
                    let mut pats = Vec::new();
                    while !self.check(TokenKind::RParen) && !self.is_at_end() {
                        pats.push(self.parse_pat()?);
                        if !self.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(TokenKind::RParen, "expected `)`")?;
                    let end = self.previous_span();
                    return Some(Pat::Enum {
                        span: start.merge(end),
                        path: TypePath {
                            span: path.span,
                            segments: path.segments,
                        },
                        fields: pats,
                    });
                }
                // Just a path pattern
                let end = self.previous_span();
                return Some(Pat::Ident {
                    span: start.merge(end),
                    name: ident,
                    is_mut: false,
                });
            }

            // Simple identifier pattern
            let end = self.previous_span();
            return Some(Pat::Ident {
                span: start.merge(end),
                name: ident,
                is_mut: false,
            });
        }

        // Tuple pattern
        if self.eat(TokenKind::LParen) {
            let mut pats = Vec::new();
            while !self.check(TokenKind::RParen) && !self.is_at_end() {
                pats.push(self.parse_pat()?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "expected `)`")?;
            let end = self.previous_span();
            return Some(Pat::Tuple {
                span: start.merge(end),
                pats,
            });
        }

        self.error(codes::PARSE_ERROR(), "expected pattern");
        None
    }

    // Expression parsing (Pratt parser)
    fn parse_expr(&mut self, precedence: Precedence) -> Option<Expr> {
        let mut left = self.parse_primary()?;

        while !self.is_at_end() {
            let token_precedence = self.current_precedence();
            if token_precedence <= precedence {
                break;
            }

            left = self.parse_infix(left)?;
        }

        Some(left)
    }

    fn current_precedence(&self) -> Precedence {
        match &self.current_token().kind {
            TokenKind::Eq => Precedence::Assignment,
            TokenKind::PlusEq
            | TokenKind::MinusEq
            | TokenKind::StarEq
            | TokenKind::SlashEq
            | TokenKind::PercentEq => Precedence::Assignment,
            TokenKind::OrOr => Precedence::Or,
            TokenKind::AndAnd => Precedence::And,
            TokenKind::EqEq | TokenKind::Ne => Precedence::Equality,
            TokenKind::Lt | TokenKind::Gt | TokenKind::Le | TokenKind::Ge => Precedence::Comparison,
            TokenKind::Plus | TokenKind::Minus => Precedence::Term,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Factor,
            TokenKind::Bang | TokenKind::Minus | TokenKind::Star => Precedence::Unary,
            TokenKind::LParen | TokenKind::LBracket | TokenKind::Dot => Precedence::Call,
            _ => Precedence::Lowest,
        }
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        let start = self.current_span();

        // Literals
        if let Some(lit) = self.parse_literal() {
            let end = self.previous_span();
            return Some(Expr::Literal {
                span: start.merge(end),
                lit,
            });
        }

        // Identifiers and paths
        if self.check(TokenKind::Ident(String::new())) {
            let ident = self.parse_ident()?;

            // Check for path (Type::Variant or module::item)
            if self.check(TokenKind::ColonColon) {
                let path = self.parse_expr_path_starting_with(ident)?;
                return Some(Expr::Path {
                    span: path.span,
                    path,
                });
            }

            // Check for call
            if self.check(TokenKind::LParen) {
                return self.parse_call_expr(Expr::Path {
                    span: ident.span,
                    path: ExprPath {
                        span: ident.span,
                        segments: vec![PathSegment {
                            span: ident.span,
                            name: ident,
                            args: None,
                        }],
                    },
                });
            }

            // Simple identifier
            let end = self.previous_span();
            return Some(Expr::Path {
                span: start.merge(end),
                path: ExprPath {
                    span: start.merge(end),
                    segments: vec![PathSegment {
                        span: ident.span,
                        name: ident,
                        args: None,
                    }],
                },
            });
        }

        // Parenthesized expression
        if self.eat(TokenKind::LParen) {
            let expr = self.parse_expr(Precedence::Lowest)?;
            self.expect(TokenKind::RParen, "expected `)`")?;
            let end = self.previous_span();
            return Some(Expr::Block {
                span: start.merge(end),
                block: Block {
                    span: start.merge(end),
                    stmts: Vec::new(),
                    tail: Some(Box::new(expr)),
                },
            });
        }

        // Tuple
        if self.eat(TokenKind::LParen) {
            let mut exprs = Vec::new();
            while !self.check(TokenKind::RParen) && !self.is_at_end() {
                exprs.push(self.parse_expr(Precedence::Lowest)?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "expected `)`")?;
            let end = self.previous_span();
            return Some(Expr::Tuple {
                span: start.merge(end),
                exprs,
            });
        }

        // Array
        if self.eat(TokenKind::LBracket) {
            let mut exprs = Vec::new();
            while !self.check(TokenKind::RBracket) && !self.is_at_end() {
                exprs.push(self.parse_expr(Precedence::Lowest)?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RBracket, "expected `]`")?;
            let end = self.previous_span();
            return Some(Expr::Array {
                span: start.merge(end),
                exprs,
            });
        }

        // Struct literal
        if self.check_ident() && self.peek(1).map_or(false, |t| t.kind == TokenKind::LBrace) {
            let ident = self.parse_ident()?;
            let path = TypePath::single(ident.span, ident);
            return self.parse_struct_expr(path, start);
        }

        // Block
        if self.check(TokenKind::LBrace) {
            let block = self.parse_block()?;
            let end = self.previous_span();
            return Some(Expr::Block {
                span: start.merge(end),
                block,
            });
        }

        // Unary operators
        if self.eat(TokenKind::Bang) {
            let expr = self.parse_expr(Precedence::Unary)?;
            let end = self.previous_span();
            return Some(Expr::Unary {
                span: start.merge(end),
                op: UnaryOp::Not,
                expr: Box::new(expr),
            });
        }

        if self.eat(TokenKind::Minus) {
            let expr = self.parse_expr(Precedence::Unary)?;
            let end = self.previous_span();
            return Some(Expr::Unary {
                span: start.merge(end),
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            });
        }

        if self.eat(TokenKind::Star) {
            let expr = self.parse_expr(Precedence::Unary)?;
            let end = self.previous_span();
            return Some(Expr::Unary {
                span: start.merge(end),
                op: UnaryOp::Deref,
                expr: Box::new(expr),
            });
        }

        // Keywords as expressions
        if self.eat(TokenKind::KwAsk) {
            return self.parse_ask_expr(start);
        }

        if self.eat(TokenKind::KwVerify) {
            return self.parse_verify_expr(start);
        }

        if self.eat(TokenKind::KwCommit) {
            return self.parse_commit_once_expr(start);
        }

        if self.eat(TokenKind::KwReturn) {
            let expr = if !self.check(TokenKind::Semi) && !self.check(TokenKind::RBrace) {
                Some(self.parse_expr(Precedence::Lowest)?)
            } else {
                None
            };
            let end = self.previous_span();
            return Some(Expr::Return {
                span: start.merge(end),
                expr: expr.map(Box::new),
            });
        }

        if self.eat(TokenKind::KwBreak) {
            let expr = if !self.check(TokenKind::Semi) && !self.check(TokenKind::RBrace) {
                Some(self.parse_expr(Precedence::Lowest)?)
            } else {
                None
            };
            let end = self.previous_span();
            return Some(Expr::Break {
                span: start.merge(end),
                expr: expr.map(Box::new),
            });
        }

        if self.eat(TokenKind::KwContinue) {
            let end = self.previous_span();
            return Some(Expr::Continue {
                span: start.merge(end),
            });
        }

        if self.eat(TokenKind::KwNew) {
            let ty = self.parse_type()?;
            self.expect(TokenKind::LParen, "expected `(`")?;
            let mut args = Vec::new();
            while !self.check(TokenKind::RParen) && !self.is_at_end() {
                args.push(self.parse_expr(Precedence::Lowest)?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "expected `)`")?;
            let end = self.previous_span();
            return Some(Expr::New {
                span: start.merge(end),
                ty,
                args,
            });
        }

        self.error(codes::PARSE_ERROR(), "expected expression");
        None
    }

    fn parse_infix(&mut self, left: Expr) -> Option<Expr> {
        let start = left.span();

        // Assignment
        if self.eat(TokenKind::Eq) {
            let right = self.parse_expr(Precedence::Assignment)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::Assign,
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        // Compound assignment
        if self.eat(TokenKind::PlusEq) {
            let right = self.parse_expr(Precedence::Assignment)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::AddAssign,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::MinusEq) {
            let right = self.parse_expr(Precedence::Assignment)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::SubAssign,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::StarEq) {
            let right = self.parse_expr(Precedence::Assignment)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::MulAssign,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::SlashEq) {
            let right = self.parse_expr(Precedence::Assignment)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::DivAssign,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::PercentEq) {
            let right = self.parse_expr(Precedence::Assignment)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::RemAssign,
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        // Binary operators
        if self.eat(TokenKind::OrOr) {
            let right = self.parse_expr(Precedence::Or)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::AndAnd) {
            let right = self.parse_expr(Precedence::And)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::And,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::EqEq) {
            let right = self.parse_expr(Precedence::Equality)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::Eq,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::Ne) {
            let right = self.parse_expr(Precedence::Equality)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::Ne,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::Lt) {
            let right = self.parse_expr(Precedence::Comparison)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::Lt,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::Gt) {
            let right = self.parse_expr(Precedence::Comparison)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::Gt,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::Le) {
            let right = self.parse_expr(Precedence::Comparison)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::Le,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::Ge) {
            let right = self.parse_expr(Precedence::Comparison)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::Ge,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::Plus) {
            let right = self.parse_expr(Precedence::Term)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::Add,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::Minus) {
            let right = self.parse_expr(Precedence::Term)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::Sub,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::Star) {
            let right = self.parse_expr(Precedence::Factor)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::Mul,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::Slash) {
            let right = self.parse_expr(Precedence::Factor)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::Div,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        if self.eat(TokenKind::Percent) {
            let right = self.parse_expr(Precedence::Factor)?;
            let end = self.previous_span();
            return Some(Expr::Binary {
                span: start.merge(end),
                op: BinaryOp::Rem,
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        // Method call
        if self.eat(TokenKind::Dot) {
            let method = self.parse_ident()?;
            if self.check(TokenKind::LParen) {
                self.eat(TokenKind::LParen);
                let mut args = Vec::new();
                while !self.check(TokenKind::RParen) && !self.is_at_end() {
                    args.push(self.parse_expr(Precedence::Lowest)?);
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::RParen, "expected `)`")?;
                let end = self.previous_span();
                return Some(Expr::MethodCall {
                    span: start.merge(end),
                    receiver: Box::new(left),
                    method,
                    args,
                });
            } else {
                // Field access
                let end = self.previous_span();
                return Some(Expr::Field {
                    span: start.merge(end),
                    base: Box::new(left),
                    field: method,
                });
            }
        }

        // Index access
        if self.eat(TokenKind::LBracket) {
            let index = self.parse_expr(Precedence::Lowest)?;
            self.expect(TokenKind::RBracket, "expected `]`")?;
            let end = self.previous_span();
            return Some(Expr::Index {
                span: start.merge(end),
                base: Box::new(left),
                index: Box::new(index),
            });
        }

        // Call
        if self.check(TokenKind::LParen) {
            return self.parse_call_expr(left);
        }

        None
    }

    fn parse_call_expr(&mut self, callee: Expr) -> Option<Expr> {
        let start = callee.span();
        self.eat(TokenKind::LParen);
        let mut args = Vec::new();
        while !self.check(TokenKind::RParen) && !self.is_at_end() {
            args.push(self.parse_expr(Precedence::Lowest)?);
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen, "expected `)`")?;
        let end = self.previous_span();
        Some(Expr::Call {
            span: start.merge(end),
            callee: Box::new(callee),
            args,
        })
    }

    fn parse_struct_expr(&mut self, path: TypePath, start: Span) -> Option<Expr> {
        self.eat(TokenKind::LBrace);
        let mut fields = Vec::new();
        let mut base = None;

        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            if self.check(TokenKind::DotDot) {
                self.eat(TokenKind::DotDot);
                base = Some(Box::new(self.parse_expr(Precedence::Lowest)?));
                break;
            }

            let field_start = self.current_span();
            let name = self.parse_ident()?;
            self.expect(TokenKind::Colon, "expected `:`")?;
            let expr = self.parse_expr(Precedence::Lowest)?;
            self.eat(TokenKind::Comma);
            let field_end = self.previous_span();
            fields.push(StructExprField {
                span: field_start.merge(field_end),
                name,
                expr,
            });
        }
        self.expect(TokenKind::RBrace, "expected `}`")?;
        let end = self.previous_span();

        Some(Expr::Struct {
            span: start.merge(end),
            path,
            fields,
            base,
        })
    }

    fn parse_ask_expr(&mut self, start: Span) -> Option<Expr> {
        self.expect(TokenKind::LParen, "expected `(` after `ask`")?;
        let model = self.parse_expr_path()?;
        self.expect(TokenKind::Comma, "expected `,`")?;
        let goal = if let Some(s) = self.parse_string_literal() {
            s
        } else {
            self.error(codes::PARSE_ERROR(), "expected string literal for goal");
            String::new()
        };
        self.expect(TokenKind::Comma, "expected `,`")?;
        let input = self.parse_expr(Precedence::Lowest)?;
        self.expect(TokenKind::Comma, "expected `,`")?;
        let output_ty = self.parse_type()?;
        self.expect(TokenKind::RParen, "expected `)`")?;
        let end = self.previous_span();

        Some(Expr::Ask {
            span: start.merge(end),
            model,
            goal,
            input: Box::new(input),
            output_ty,
        })
    }

    fn parse_verify_expr(&mut self, start: Span) -> Option<Expr> {
        self.expect(TokenKind::LParen, "expected `(` after `verify`")?;

        // Handle empty args case explicitly to avoid backtracking
        if self.check(TokenKind::RParen) {
            self.error(
                codes::PARSE_ERROR(),
                "verify expects 2 arguments (claim, policy), received 0",
            );
            self.eat(TokenKind::RParen);
            let end = self.previous_span();
            return Some(Expr::Verify {
                span: start.merge(end),
                claim: Box::new(Expr::Literal {
                    span: start,
                    lit: Literal::Unit { span: start },
                }),
                policy: TypePath::single(start, Ident::dummy("")),
            });
        }

        let claim = self.parse_expr(Precedence::Lowest)?;

        if self.check(TokenKind::RParen) {
            self.error(
                codes::PARSE_ERROR(),
                "verify expects 2 arguments (claim, policy), received 1",
            );
            self.eat(TokenKind::RParen);
            let end = self.previous_span();
            return Some(Expr::Verify {
                span: start.merge(end),
                claim: Box::new(claim),
                policy: TypePath::single(start, Ident::dummy("")),
            });
        }

        self.expect(TokenKind::Comma, "expected `,`")?;

        if self.check(TokenKind::RParen) {
            self.error(
                codes::PARSE_ERROR(),
                "verify expects 2 arguments (claim, policy), received 1",
            );
            self.eat(TokenKind::RParen);
            let end = self.previous_span();
            return Some(Expr::Verify {
                span: start.merge(end),
                claim: Box::new(claim),
                policy: TypePath::single(start, Ident::dummy("")),
            });
        }

        let policy = self.parse_type_path()?;

        // Check for extra arguments after policy
        while !self.check(TokenKind::RParen) && !self.is_at_end() {
            self.error(
                codes::PARSE_ERROR(),
                "verify expects exactly 2 arguments, found extra argument",
            );
            self.eat(TokenKind::Comma); // consume comma if present
            self.parse_expr(Precedence::Lowest)?; // consume extra arg
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }

        self.expect(TokenKind::RParen, "expected `)`")?;
        let end = self.previous_span();

        Some(Expr::Verify {
            span: start.merge(end),
            claim: Box::new(claim),
            policy,
        })
    }

    fn parse_commit_once_expr(&mut self, start: Span) -> Option<Expr> {
        self.expect(TokenKind::KwOnce, "expected `once` after `commit`")?;
        self.expect(TokenKind::LParen, "expected `(` after `commit once`")?;
        let effect = self.parse_effect_ref()?;
        self.expect(TokenKind::Comma, "expected `,`")?;
        let mut args = Vec::new();
        while !self.check(TokenKind::RParen) && !self.is_at_end() {
            args.push(self.parse_expr(Precedence::Lowest)?);
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen, "expected `)`")?;
        let end = self.previous_span();

        Some(Expr::CommitOnce {
            span: start.merge(end),
            effect,
            args,
        })
    }

    // Type parsing
    fn parse_type(&mut self) -> Option<Type> {
        let start = self.current_span();

        // Unit type
        if self.eat(TokenKind::LParen) && self.check(TokenKind::RParen) {
            self.eat(TokenKind::RParen);
            let end = self.previous_span();
            return Some(Type::Unit {
                span: start.merge(end),
            });
        }

        // Never type
        if self.eat(TokenKind::Bang) {
            let end = self.previous_span();
            return Some(Type::Never {
                span: start.merge(end),
            });
        }

        // Reference type
        if self.eat(TokenKind::And) {
            let is_mut = self.eat(TokenKind::KwMut);
            let ty = self.parse_type()?;
            let end = self.previous_span();
            return Some(Type::Ref {
                span: start.merge(end),
                is_mut,
                ty: Box::new(ty),
            });
        }

        // Owned type
        if self.eat(TokenKind::KwOwned) {
            let ty = self.parse_type()?;
            let end = self.previous_span();
            return Some(Type::Owned {
                span: start.merge(end),
                ty: Box::new(ty),
            });
        }

        // Claim type
        if self.eat(TokenKind::KwClaim) {
            self.expect(TokenKind::Lt, "expected `<` after `Claim`")?;
            let ty = self.parse_type()?;
            self.expect(TokenKind::Gt, "expected `>`")?;
            let end = self.previous_span();
            return Some(Type::Claim {
                span: start.merge(end),
                ty: Box::new(ty),
            });
        }

        // Verified type
        if self.eat(TokenKind::KwVerified) {
            self.expect(TokenKind::Lt, "expected `<` after `Verified`")?;
            let ty = self.parse_type()?;
            self.expect(TokenKind::Comma, "expected `,`")?;
            let policy = self.parse_type()?;
            self.expect(TokenKind::Gt, "expected `>`")?;
            let end = self.previous_span();
            return Some(Type::Verified {
                span: start.merge(end),
                ty: Box::new(ty),
                policy: Box::new(policy),
            });
        }

        // Function type
        if self.eat(TokenKind::KwFn) {
            self.expect(TokenKind::LParen, "expected `(`")?;
            let mut params = Vec::new();
            while !self.check(TokenKind::RParen) && !self.is_at_end() {
                params.push(self.parse_type()?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "expected `)`")?;
            self.expect(TokenKind::Arrow, "expected `->`")?;
            let ret = self.parse_type()?;
            let effects = if self.eat(TokenKind::KwUses) {
                let mut effects = Vec::new();
                while !self.check(TokenKind::LBrace) && !self.is_at_end() {
                    effects.push(self.parse_effect_ref()?);
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                EffectSet {
                    span: self.previous_span(),
                    effects,
                }
            } else {
                EffectSet::default()
            };
            let end = self.previous_span();
            return Some(Type::Fn {
                span: start.merge(end),
                params,
                ret: Box::new(ret),
                effects,
            });
        }

        // Array type
        if self.eat(TokenKind::LBracket) {
            let ty = self.parse_type()?;
            let size = if self.eat(TokenKind::Semi) {
                Some(Box::new(self.parse_expr(Precedence::Lowest)?))
            } else {
                None
            };
            self.expect(TokenKind::RBracket, "expected `]`")?;
            let end = self.previous_span();
            return Some(Type::Array {
                span: start.merge(end),
                ty: Box::new(ty),
                size,
            });
        }

        // Tuple type
        if self.eat(TokenKind::LParen) {
            let mut types = Vec::new();
            while !self.check(TokenKind::RParen) && !self.is_at_end() {
                types.push(self.parse_type()?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "expected `)`")?;
            let end = self.previous_span();
            return Some(Type::Tuple {
                span: start.merge(end),
                types,
            });
        }

        // Path type (including primitives and user types)
        let path = self.parse_type_path()?;
        let end = self.previous_span();

        // Check for primitive types
        if let Some(ident) = path.as_ident() {
            match ident.name.as_str() {
                "bool" => {
                    return Some(Type::Bool {
                        span: start.merge(end),
                    });
                }
                "int" => {
                    return Some(Type::Int {
                        span: start.merge(end),
                    });
                }
                "float" => {
                    return Some(Type::Float {
                        span: start.merge(end),
                    });
                }
                "string" => {
                    return Some(Type::String {
                        span: start.merge(end),
                    });
                }
                _ => {}
            }
        }

        Some(Type::Path {
            span: start.merge(end),
            path,
        })
    }

    fn parse_type_path(&mut self) -> Option<TypePath> {
        let start = self.current_span();
        let mut segments = Vec::new();

        loop {
            let ident = self.parse_ident()?;
            let args = if self.eat(TokenKind::Lt) {
                let mut args = Vec::new();
                while !self.check(TokenKind::Gt) && !self.is_at_end() {
                    args.push(self.parse_generic_arg()?);
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::Gt, "expected `>`")?;
                Some(GenericArgs {
                    span: ident.span.merge(self.previous_span()),
                    args,
                })
            } else {
                None
            };

            segments.push(PathSegment {
                span: ident.span,
                name: ident,
                args,
            });

            if !self.eat(TokenKind::ColonColon) {
                break;
            }
        }

        let end = self.previous_span();
        Some(TypePath {
            span: start.merge(end),
            segments,
        })
    }

    fn parse_expr_path(&mut self) -> Option<ExprPath> {
        let start = self.current_span();
        let mut segments = Vec::new();

        loop {
            let ident = self.parse_ident()?;
            let args = if self.eat(TokenKind::Lt) {
                let mut args = Vec::new();
                while !self.check(TokenKind::Gt) && !self.is_at_end() {
                    args.push(self.parse_generic_arg()?);
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::Gt, "expected `>`")?;
                Some(GenericArgs {
                    span: ident.span.merge(self.previous_span()),
                    args,
                })
            } else {
                None
            };

            segments.push(PathSegment {
                span: ident.span,
                name: ident,
                args,
            });

            if !self.eat(TokenKind::ColonColon) {
                break;
            }
        }

        let end = self.previous_span();
        Some(ExprPath {
            span: start.merge(end),
            segments,
        })
    }

    fn parse_expr_path_starting_with(&mut self, first: Ident) -> Option<ExprPath> {
        let start = first.span;
        let mut segments = vec![PathSegment {
            span: first.span,
            name: first,
            args: None,
        }];

        while self.eat(TokenKind::ColonColon) {
            let ident = self.parse_ident()?;
            let args = if self.eat(TokenKind::Lt) {
                let mut args = Vec::new();
                while !self.check(TokenKind::Gt) && !self.is_at_end() {
                    args.push(self.parse_generic_arg()?);
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::Gt, "expected `>`")?;
                Some(GenericArgs {
                    span: ident.span.merge(self.previous_span()),
                    args,
                })
            } else {
                None
            };

            segments.push(PathSegment {
                span: ident.span,
                name: ident,
                args,
            });
        }

        let end = self.previous_span();
        Some(ExprPath {
            span: start.merge(end),
            segments,
        })
    }

    fn parse_generic_arg(&mut self) -> Option<GenericArg> {
        if self.check_ident() || self.check(TokenKind::KwFn) {
            Some(GenericArg::Type {
                span: self.current_span(),
                ty: self.parse_type()?,
            })
        } else {
            Some(GenericArg::Const {
                span: self.current_span(),
                expr: self.parse_expr(Precedence::Lowest)?,
            })
        }
    }

    // Helpers
    fn parse_ident(&mut self) -> Option<Ident> {
        if let TokenKind::Ident(name) = &self.current_token().kind {
            let span = self.current_token().span;
            let ident = Ident::new(span, name.clone());
            self.advance();
            Some(ident)
        } else {
            self.error(codes::PARSE_ERROR(), "expected identifier");
            None
        }
    }

    fn parse_literal(&mut self) -> Option<Literal> {
        let token = self.current_token().clone();
        let span = token.span;

        let lit = match token.kind {
            TokenKind::String(s) => Literal::String { span, value: s },
            TokenKind::Int(i) => Literal::Int { span, value: i },
            TokenKind::Float(f) => Literal::Float { span, value: f },
            TokenKind::Bool(b) => Literal::Bool { span, value: b },
            _ => return None,
        };

        self.advance();
        Some(lit)
    }

    fn parse_string_literal(&mut self) -> Option<String> {
        if let TokenKind::String(s) = &self.current_token().kind {
            let s = s.clone();
            self.advance();
            Some(s)
        } else {
            None
        }
    }

    fn current_token(&self) -> &Token {
        self.tokens.get(self.current).unwrap_or_else(|| {
            EMPTY_TOKEN.get_or_init(|| {
                Token::new(
                    TokenKind::Ident(String::new()),
                    Span::new(FileId::new(0), ByteOffset(0), ByteOffset(0)),
                )
            })
        })
    }

    fn peek(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.current + offset)
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous_token()
    }

    fn previous_token(&self) -> &Token {
        &self.tokens[self.current.saturating_sub(1)]
    }

    fn previous_span(&self) -> Span {
        self.previous_token().span
    }

    fn current_span(&self) -> Span {
        self.current_token().span
    }

    fn check(&self, kind: TokenKind) -> bool {
        !self.is_at_end()
            && std::mem::discriminant(&self.current_token().kind) == std::mem::discriminant(&kind)
    }

    fn check_ident(&self) -> bool {
        !self.is_at_end() && self.current_token().kind.is_ident()
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind, msg: &str) -> Option<&Token> {
        if self.check(kind) {
            Some(self.advance())
        } else {
            self.error(codes::PARSE_ERROR(), msg);
            None
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    fn error(&mut self, code: crate::diagnostic::DiagnosticCode, message: &str) {
        let span = self.current_span();
        let diag = DiagnosticBuilder::error(code, message)
            .primary_label(span, "here")
            .build();
        self.diagnostics.push(diag);
    }

    fn skip_to_next_item(&mut self) {
        while !self.is_at_end() {
            if matches!(
                self.current_token().kind,
                TokenKind::KwFn
                    | TokenKind::KwStruct
                    | TokenKind::KwEnum
                    | TokenKind::KwType
                    | TokenKind::KwUse
                    | TokenKind::KwMod
                    | TokenKind::KwPolicy
                    | TokenKind::KwClaim
                    | TokenKind::KwEffect
            ) {
                break;
            }
            self.advance();
        }
    }
}

/// Parse a source file into an AST.
pub fn parse(tokens: &[Token], file_id: FileId) -> (Module, Diagnostics) {
    let parser = Parser::new(tokens, file_id);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::span::FileId;

    #[test]
    fn test_parse_simple_fn() {
        let source = r#"
fn main() -> int {
    return 42;
}
"#;
        let tokens = lex(source, FileId::new(0));
        let (module, diagnostics) = parse(&tokens, FileId::new(0));
        assert!(!diagnostics.has_errors());
        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn test_parse_claim_verified() {
        let source = r#"
    fn refund(claim: Claim<RefundDecision>) -> Receipt
    uses PaymentGateway:
        { return payments.refund(claim); }
    "#;
        let tokens = lex(source, FileId::new(0));
        let (module, diagnostics) = parse(&tokens, FileId::new(0));
        // Should parse without syntax errors
        assert!(!diagnostics.has_errors());
    }
}
