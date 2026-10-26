//! CST to AST lowering: shapes, names, labels, attributes, errors.

use vba_ast::*;
use vba_profile::find_profile;
use vba_source::FileId;
use vba_syntax::ParseOptions;

fn parse(src: &str) -> ParsedModule {
    parse_module(
        FileId(0),
        src.as_bytes(),
        find_profile("vba7.1-excel-win64").unwrap(),
        &ParseOptions::tolerant(),
    )
}

fn clean(src: &str) -> ParsedModule {
    let p = parse(src);
    assert!(p.diagnostics.is_empty(), "{src:?}: {:?}", p.diagnostics);
    p
}

/// Statements of the single procedure wrapping `body`.
fn body(body: &str) -> Vec<Stmt> {
    let p = clean(&format!("Sub T()\n{body}\nEnd Sub\n"));
    let proc_ = p.module.procs().next().expect("proc").clone();
    proc_.body.stmts
}

fn only(body_src: &str) -> Stmt {
    let mut s = body(body_src);
    assert_eq!(s.len(), 1, "{body_src}: {s:?}");
    s.remove(0)
}

fn rhs(e: &str) -> Expr {
    match only(&format!("x = {e}")).kind {
        StmtKind::Assign { value, .. } => value,
        k => panic!("{k:?}"),
    }
}

// ---- names ----------------------------------------------------------------

#[test]
fn names_are_case_insensitive_and_keep_spelling() {
    use std::collections::HashSet;
    let a = Name::new("FooBar");
    let b = Name::new("fOObAR");
    assert_eq!(a, b);
    assert_eq!(a.as_bytes(), b"FooBar");
    assert_eq!(b.to_string_lossy(), "fOObAR");
    let mut set = HashSet::new();
    set.insert(a.clone());
    assert!(set.contains(&b));
    assert_ne!(a, Name::new("FooBaz"));
    assert!(a.eq_ignore_case("FOOBAR"));
    assert_eq!(a.folded(), b"foobar");
    assert!(Name::new("a") < Name::new("B"));

    let p = clean("Sub Hello()\nDim MyVar As Long\nmyvar = 1\nMYVAR = 2\nEnd Sub\n");
    let proc_ = p.module.procs().next().unwrap();
    let names: Vec<Name> = proc_
        .body
        .stmts
        .iter()
        .filter_map(|s| match &s.kind {
            StmtKind::Assign { target, .. } => match &target.kind {
                ExprKind::Name(i) => Some(i.name.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(names, vec![Name::new("MyVar"), Name::new("MyVar")]);
    assert_eq!(
        names
            .iter()
            .map(|n| n.to_string_lossy())
            .collect::<Vec<_>>(),
        ["myvar", "MYVAR"]
    );
}

#[test]
fn idents_split_suffix_and_foreign_names() {
    match rhs("s$").kind {
        ExprKind::Name(i) => {
            assert_eq!(i.name.to_string_lossy(), "s");
            assert_eq!(i.suffix, Some(vba_lexer::TypeSuffix::String));
            assert_eq!(i.kind, IdentKind::Plain);
        }
        k => panic!("{k:?}"),
    }
    match rhs("[Some Name]").kind {
        ExprKind::Name(i) => {
            assert_eq!(i.name.to_string_lossy(), "Some Name");
            assert_eq!(i.kind, IdentKind::Foreign);
        }
        k => panic!("{k:?}"),
    }
    match rhs("Len(a)").kind {
        ExprKind::Call { callee, .. } => match callee.kind {
            ExprKind::Name(i) => assert_eq!(i.kind, IdentKind::Keyword(vba_lexer::Keyword::Len)),
            k => panic!("{k:?}"),
        },
        k => panic!("{k:?}"),
    }
}

// ---- expressions ------------------------------------------------------------

#[test]
fn literals() {
    assert!(matches!(
        rhs("42").kind,
        ExprKind::Literal(Literal::Integer {
            value: Some(42),
            radix: Radix::Decimal,
            suffix: None
        })
    ));
    assert!(matches!(
        rhs("&HFF&").kind,
        ExprKind::Literal(Literal::Integer {
            value: Some(255),
            radix: Radix::Hex,
            suffix: Some(vba_lexer::TypeSuffix::Long)
        })
    ));
    assert!(matches!(
        rhs("&O17").kind,
        ExprKind::Literal(Literal::Integer {
            value: Some(15),
            radix: Radix::Octal,
            ..
        })
    ));
    assert!(matches!(
        rhs("&17").kind,
        ExprKind::Literal(Literal::Integer {
            value: Some(15),
            radix: Radix::Octal,
            ..
        })
    ));
    match rhs("1.5E2").kind {
        ExprKind::Literal(Literal::Float { value, .. }) => assert_eq!(value, 150.0),
        k => panic!("{k:?}"),
    }
    match rhs("1.5D2#").kind {
        ExprKind::Literal(Literal::Float { value, suffix, .. }) => {
            assert_eq!(value, 150.0);
            assert_eq!(suffix, Some(vba_lexer::TypeSuffix::Double));
        }
        k => panic!("{k:?}"),
    }
    assert_eq!(
        rhs("\"a\"\"b\"").kind,
        ExprKind::Literal(Literal::String(b"a\"b".to_vec()))
    );
    assert_eq!(
        rhs("#1/2/2020#").kind,
        ExprKind::Literal(Literal::Date(b"1/2/2020".to_vec()))
    );
    assert_eq!(rhs("True").kind, ExprKind::Literal(Literal::True));
    assert_eq!(rhs("Nothing").kind, ExprKind::Literal(Literal::Nothing));
    assert_eq!(rhs("Null").kind, ExprKind::Literal(Literal::Null));
    assert!(matches!(
        rhs("99999999999999999999999").kind,
        ExprKind::Literal(Literal::Integer { value: None, .. })
    ));
}

#[test]
fn operators_and_structure() {
    match rhs("a + b * c").kind {
        ExprKind::Binary {
            op: BinaryOp::Add,
            rhs,
            ..
        } => {
            assert!(matches!(
                rhs.kind,
                ExprKind::Binary {
                    op: BinaryOp::Mul,
                    ..
                }
            ));
        }
        k => panic!("{k:?}"),
    }
    assert!(matches!(
        rhs("-a ^ 2").kind,
        ExprKind::Unary {
            op: UnaryOp::Neg,
            ..
        }
    ));
    assert!(matches!(
        rhs("Not a").kind,
        ExprKind::Unary {
            op: UnaryOp::Not,
            ..
        }
    ));
    assert!(matches!(rhs("(a)").kind, ExprKind::Paren(_)));
    assert!(matches!(
        rhs("a Like b").kind,
        ExprKind::Binary {
            op: BinaryOp::Like,
            ..
        }
    ));
    assert!(matches!(
        rhs("a Is b").kind,
        ExprKind::Binary {
            op: BinaryOp::Is,
            ..
        }
    ));
    assert!(matches!(
        rhs("a Imp b").kind,
        ExprKind::Binary {
            op: BinaryOp::Imp,
            ..
        }
    ));
    assert!(matches!(
        rhs("a <> b").kind,
        ExprKind::Binary {
            op: BinaryOp::Ne,
            ..
        }
    ));
    assert!(matches!(
        rhs("a \\ b").kind,
        ExprKind::Binary {
            op: BinaryOp::IntDiv,
            ..
        }
    ));
    assert!(matches!(rhs("New Foo").kind, ExprKind::New(_)));
    assert!(matches!(rhs("AddressOf f").kind, ExprKind::AddressOf(_)));
    assert!(matches!(
        rhs("TypeOf a Is Foo").kind,
        ExprKind::TypeOfIs { .. }
    ));
    assert!(matches!(
        rhs("a.b").kind,
        ExprKind::Member {
            target: Some(_),
            ..
        }
    ));
    assert!(matches!(
        rhs(".b").kind,
        ExprKind::Member { target: None, .. }
    ));
    assert!(matches!(
        rhs("a!b").kind,
        ExprKind::Dict {
            target: Some(_),
            ..
        }
    ));
    match rhs("f(1, , x:=3)").kind {
        ExprKind::Call { args, .. } => {
            assert_eq!(args.len(), 3);
            assert!(args[0].value.is_some() && args[0].name.is_none());
            assert!(args[1].value.is_none());
            assert_eq!(args[2].name.as_ref().unwrap().name, Name::new("X"));
        }
        k => panic!("{k:?}"),
    }
}

// ---- statements ----------------------------------------------------------------

#[test]
fn assignments_and_calls() {
    assert!(matches!(
        only("Let x = 1").kind,
        StmtKind::Assign {
            kind: AssignKind::Let,
            ..
        }
    ));
    assert!(matches!(
        only("LSet a = b").kind,
        StmtKind::Assign {
            kind: AssignKind::LSet,
            ..
        }
    ));
    assert!(matches!(only("Set a = Nothing").kind, StmtKind::Set { .. }));
    match only("Foo 1, 2").kind {
        StmtKind::Call {
            args,
            explicit: false,
            parenthesized: false,
            ..
        } => {
            assert_eq!(args.len(), 2)
        }
        k => panic!("{k:?}"),
    }
    match only("Foo(1, 2)").kind {
        StmtKind::Call {
            args,
            explicit: false,
            parenthesized: true,
            callee,
        } => {
            assert_eq!(args.len(), 2);
            assert!(matches!(callee.kind, ExprKind::Name(_)));
        }
        k => panic!("{k:?}"),
    }
    match only("Call obj.Run(1)").kind {
        StmtKind::Call {
            explicit: true,
            parenthesized: true,
            callee,
            args,
        } => {
            assert!(matches!(callee.kind, ExprKind::Member { .. }));
            assert_eq!(args.len(), 1);
        }
        k => panic!("{k:?}"),
    }
    // `Foo (a)` passes a parenthesised (by value) argument.
    match only("Foo (a)").kind {
        StmtKind::Call {
            args,
            parenthesized: false,
            ..
        } => {
            assert!(matches!(
                args[0].value.as_ref().unwrap().kind,
                ExprKind::Paren(_)
            ));
        }
        k => panic!("{k:?}"),
    }
}

#[test]
fn if_forms() {
    match only("If a Then b = 1 Else c = 2").kind {
        StmtKind::If(i) => {
            assert!(i.single_line);
            assert_eq!(i.then_body.stmts.len(), 1);
            assert_eq!(i.else_body.unwrap().stmts.len(), 1);
        }
        k => panic!("{k:?}"),
    }
    match only("If a Then\nx = 1\nElseIf b Then\nx = 2\nElseIf c Then\nx = 3\nElse\nx = 4\nEnd If")
        .kind
    {
        StmtKind::If(i) => {
            assert!(!i.single_line);
            assert_eq!(i.else_ifs.len(), 2);
            assert!(i.else_body.is_some());
        }
        k => panic!("{k:?}"),
    }
    match only("If a Then 100").kind {
        StmtKind::If(i) => match &i.then_body.stmts[0].kind {
            StmtKind::GoTo(LabelRef::LineNumber {
                value: Some(100), ..
            }) => {}
            k => panic!("{k:?}"),
        },
        k => panic!("{k:?}"),
    }
}

#[test]
fn select_case() {
    match only("Select Case x\nCase 1, 2 To 3, Is >= 4\ny = 1\nCase Else\ny = 2\nEnd Select").kind {
        StmtKind::Select(s) => {
            assert_eq!(s.cases.len(), 2);
            assert_eq!(s.cases[0].items.len(), 3);
            assert!(matches!(s.cases[0].items[0], CaseItem::Value(_)));
            assert!(matches!(s.cases[0].items[1], CaseItem::Range { .. }));
            assert!(matches!(
                s.cases[0].items[2],
                CaseItem::Is {
                    op: BinaryOp::Ge,
                    ..
                }
            ));
            assert!(s.cases[1].is_else && s.cases[1].items.is_empty());
        }
        k => panic!("{k:?}"),
    }
}

#[test]
fn loops() {
    match only("For i = 1 To 10 Step 2\nNext i").kind {
        StmtKind::For(f) => {
            assert!(f.step.is_some());
            assert_eq!(f.next_counters.len(), 1);
        }
        k => panic!("{k:?}"),
    }
    match only("For Each c In coll\nNext").kind {
        StmtKind::ForEach(f) => assert!(f.next_counters.is_empty()),
        k => panic!("{k:?}"),
    }
    match only("Do While a\nLoop").kind {
        StmtKind::Do(d) => {
            assert_eq!(d.pre.as_ref().unwrap().kind, LoopCondKind::While);
            assert!(d.post.is_none());
        }
        k => panic!("{k:?}"),
    }
    match only("Do\nLoop Until a").kind {
        StmtKind::Do(d) => {
            assert!(d.pre.is_none());
            assert_eq!(d.post.as_ref().unwrap().kind, LoopCondKind::Until);
        }
        k => panic!("{k:?}"),
    }
    assert!(matches!(only("While a\nWend").kind, StmtKind::While { .. }));
    assert!(matches!(
        only("With a\nEnd With").kind,
        StmtKind::With { .. }
    ));
    // `Next j, i` closes two loops: the inner For lists both counters.
    match only("For i = 1 To 2\nFor j = 1 To 2\nNext j, i").kind {
        StmtKind::For(outer) => match &outer.body.stmts[0].kind {
            StmtKind::For(inner) => assert_eq!(inner.next_counters.len(), 2),
            k => panic!("{k:?}"),
        },
        k => panic!("{k:?}"),
    }
}

#[test]
fn labels_and_jumps() {
    let s = body(
        "10 x = 1\nlbl:\nGoTo lbl\nGoSub 10\nOn Error GoTo lbl\nOn Error GoTo 0\nOn Error GoTo -1\nOn Error Resume Next\nOn x GoTo lbl, 10\nResume\nResume Next\nResume lbl\nReturn\nExit Sub\nExit Property\nStop\nEnd",
    );
    let kinds: Vec<&StmtKind> = s.iter().map(|s| &s.kind).collect();
    assert!(matches!(
        kinds[0],
        StmtKind::Label(LabelDef {
            label: LabelRef::LineNumber {
                value: Some(10),
                ..
            },
            ..
        })
    ));
    assert!(matches!(kinds[1], StmtKind::Assign { .. }));
    assert!(matches!(
        kinds[2],
        StmtKind::Label(LabelDef {
            label: LabelRef::Name(_),
            ..
        })
    ));
    assert!(matches!(kinds[3], StmtKind::GoTo(LabelRef::Name(_))));
    assert!(matches!(
        kinds[4],
        StmtKind::GoSub(LabelRef::LineNumber { .. })
    ));
    assert!(matches!(
        kinds[5],
        StmtKind::OnError(OnErrorAction::GoTo(_))
    ));
    assert!(matches!(
        kinds[6],
        StmtKind::OnError(OnErrorAction::GoToZero)
    ));
    assert!(matches!(
        kinds[7],
        StmtKind::OnError(OnErrorAction::GoToMinusOne)
    ));
    assert!(matches!(
        kinds[8],
        StmtKind::OnError(OnErrorAction::ResumeNext)
    ));
    match kinds[9] {
        StmtKind::OnGoTo {
            targets,
            is_gosub: false,
            ..
        } => assert_eq!(targets.len(), 2),
        k => panic!("{k:?}"),
    }
    assert!(matches!(kinds[10], StmtKind::Resume(ResumeTarget::Same)));
    assert!(matches!(kinds[11], StmtKind::Resume(ResumeTarget::Next)));
    assert!(matches!(
        kinds[12],
        StmtKind::Resume(ResumeTarget::Label(_))
    ));
    assert!(matches!(kinds[13], StmtKind::Return));
    assert!(matches!(kinds[14], StmtKind::Exit(ExitKind::Sub)));
    assert!(matches!(kinds[15], StmtKind::Exit(ExitKind::Property)));
    assert!(matches!(kinds[16], StmtKind::Stop));
    assert!(matches!(kinds[17], StmtKind::End));

    let p = clean("Sub T()\nlbl:\n10 x = 1\nGoTo lbl\nEnd Sub\n");
    let proc_ = p.module.procs().next().unwrap();
    let labels: Vec<_> = proc_.body.labels().collect();
    assert_eq!(labels.len(), 2);
    let goto = match &proc_.body.stmts[3].kind {
        StmtKind::GoTo(l) => l,
        k => panic!("{k:?}"),
    };
    assert!(labels[0].label.same_target(goto));
    assert!(!labels[1].label.same_target(goto));
}

#[test]
fn declarations_in_procedures() {
    match only("Dim a(1 To 3) As Integer, b$, c As New Collection").kind {
        StmtKind::Dim(d) => {
            assert_eq!(d.declarators.len(), 3);
            let a = &d.declarators[0];
            let bounds = a.bounds.as_ref().unwrap();
            assert_eq!(bounds.len(), 1);
            assert!(bounds[0].lower.is_some());
            assert_eq!(
                a.as_clause.as_ref().unwrap().ty.kind,
                TypeRefKind::Builtin(BuiltinType::Integer)
            );
            assert_eq!(
                d.declarators[1].name.suffix,
                Some(vba_lexer::TypeSuffix::String)
            );
            assert!(d.declarators[2].as_clause.as_ref().unwrap().is_new);
        }
        k => panic!("{k:?}"),
    }
    match only("Static n As Long").kind {
        StmtKind::Dim(d) => {
            assert!(d.is_static);
            assert_eq!(d.keyword, VarKeyword::Static);
        }
        k => panic!("{k:?}"),
    }
    match only("Const K As Long = 3, J = 4").kind {
        StmtKind::Const(c) => assert_eq!(c.items.len(), 2),
        k => panic!("{k:?}"),
    }
    match only("ReDim Preserve a(1 To n) As Long, b(5)").kind {
        StmtKind::ReDim {
            preserve: true,
            items,
        } => {
            assert_eq!(items.len(), 2);
            assert!(matches!(items[0].target.kind, ExprKind::Name(_)));
            assert_eq!(items[0].bounds.len(), 1);
            assert!(items[0].bounds[0].lower.is_some());
            assert!(items[0].as_clause.is_some());
            assert!(items[1].bounds[0].lower.is_none());
        }
        k => panic!("{k:?}"),
    }
    assert!(matches!(only("Erase a, b").kind, StmtKind::Erase(v) if v.len() == 2));
    match only("Dim s As String * 10").kind {
        StmtKind::Dim(d) => assert!(
            d.declarators[0]
                .as_clause
                .as_ref()
                .unwrap()
                .fixed_length
                .is_some()
        ),
        k => panic!("{k:?}"),
    }
}

#[test]
fn file_statements() {
    match only("Open \"f\" For Binary Access Read Write Lock Read As #1 Len = 8").kind {
        StmtKind::Open(o) => {
            assert_eq!(o.mode, Some(OpenMode::Binary));
            assert_eq!(o.access, Some(OpenAccess::ReadWrite));
            assert_eq!(o.lock, Some(OpenLock::LockRead));
            assert!(matches!(o.file.kind, ExprKind::Literal(_)));
            assert!(o.record_len.is_some());
        }
        k => panic!("{k:?}"),
    }
    match only("Open p For Input Shared As #n").kind {
        StmtKind::Open(o) => {
            assert_eq!(o.mode, Some(OpenMode::Input));
            assert_eq!(o.lock, Some(OpenLock::Shared));
            assert!(o.access.is_none());
        }
        k => panic!("{k:?}"),
    }
    match only("Open p As #1").kind {
        StmtKind::Open(o) => assert!(o.mode.is_none()),
        k => panic!("{k:?}"),
    }
    match only("Print #1, a; b, Tab(5)").kind {
        StmtKind::Print { items, .. } => {
            assert_eq!(items.len(), 5);
            assert!(matches!(items[1], OutputItem::Semicolon));
            assert!(matches!(items[3], OutputItem::Comma));
        }
        k => panic!("{k:?}"),
    }
    assert!(matches!(
        only("Write #1, a, b").kind,
        StmtKind::Write { .. }
    ));
    assert!(
        matches!(only("Input #1, a, b").kind, StmtKind::Input { targets, .. } if targets.len() == 2)
    );
    assert!(matches!(
        only("Line Input #1, s").kind,
        StmtKind::LineInput { .. }
    ));
    assert!(matches!(
        only("Get #1, , x").kind,
        StmtKind::Get {
            record: None,
            target: Some(_),
            ..
        }
    ));
    assert!(matches!(
        only("Put #1, 5, x").kind,
        StmtKind::Put {
            record: Some(_),
            value: Some(_),
            ..
        }
    ));
    assert!(matches!(
        only("Seek #1, 10").kind,
        StmtKind::Seek {
            position: Some(_),
            ..
        }
    ));
    assert!(matches!(
        only("Lock #1, 1 To 5").kind,
        StmtKind::Lock {
            from: Some(_),
            to: Some(_),
            ..
        }
    ));
    assert!(matches!(
        only("Unlock #1, 3").kind,
        StmtKind::Unlock {
            from: Some(_),
            to: None,
            ..
        }
    ));
    assert!(matches!(
        only("Width #1, 80").kind,
        StmtKind::Width { width: Some(_), .. }
    ));
    assert!(matches!(only("Close #1, #2").kind, StmtKind::Close(v) if v.len() == 2));
    assert!(matches!(only("Close").kind, StmtKind::Close(v) if v.is_empty()));
    assert!(matches!(only("Name a As b").kind, StmtKind::Name { .. }));
    assert!(matches!(only("Debug.Print a; b").kind, StmtKind::DebugPrint(v) if v.len() == 3));
    assert!(
        matches!(only("RaiseEvent Changed(1)").kind, StmtKind::RaiseEvent { args, .. } if args.len() == 1)
    );
}

// ---- declarations ---------------------------------------------------------------

#[test]
fn module_declarations() {
    let p = clean(
        "Attribute VB_Name = \"M\"\nOption Explicit\nOption Base 1\nOption Compare Text\nOption Private Module\n\
         DefInt A-C, X\nImplements IFoo\nEvent Changed(ByVal x As Long)\nPublic Const A = 1\n\
         Private x As Long\nPublic Type T\ny As Long\nEnd Type\nPublic Enum E\nA1\nB1 = 5\nEnd Enum\n\
         Private Declare PtrSafe Function F Lib \"k\" Alias \"FA\" (ByVal a As LongPtr) As Long\n\
         Public Static Function G(Optional ByRef a, ParamArray b() As Variant) As String()\nEnd Function\n\
         Property Get P() As Long\nEnd Property\nProperty Let P(v As Long)\nEnd Property\nProperty Set Q(v As Object)\nEnd Property\n",
    );
    let m = &p.module;
    assert_eq!(m.vb_name(), Some(Name::new("m")));
    let mut it = m.items.iter();
    assert!(matches!(it.next(), Some(Item::Attribute(_))));
    assert!(matches!(
        it.next(),
        Some(Item::Option(OptionDecl {
            kind: Some(OptionKind::Explicit),
            ..
        }))
    ));
    assert!(matches!(
        it.next(),
        Some(Item::Option(OptionDecl {
            kind: Some(OptionKind::Base(1)),
            ..
        }))
    ));
    assert!(matches!(
        it.next(),
        Some(Item::Option(OptionDecl {
            kind: Some(OptionKind::Compare(CompareMode::Text)),
            ..
        }))
    ));
    assert!(matches!(
        it.next(),
        Some(Item::Option(OptionDecl {
            kind: Some(OptionKind::PrivateModule),
            ..
        }))
    ));
    match it.next() {
        Some(Item::DefType(d)) => {
            assert_eq!(d.ty, DefType::Int);
            assert_eq!(d.ranges, vec![(b'A', b'C'), (b'X', b'X')]);
        }
        i => panic!("{i:?}"),
    }
    assert!(matches!(it.next(), Some(Item::Implements(_))));
    match it.next() {
        Some(Item::Event(e)) => assert_eq!(e.params.len(), 1),
        i => panic!("{i:?}"),
    }
    match it.next() {
        Some(Item::Const(c)) => assert_eq!(c.visibility, Some(Visibility::Public)),
        i => panic!("{i:?}"),
    }
    match it.next() {
        Some(Item::Var(v)) => assert_eq!(v.visibility, Some(Visibility::Private)),
        i => panic!("{i:?}"),
    }
    match it.next() {
        Some(Item::Type(t)) => {
            assert_eq!(t.name.name, Name::new("t"));
            assert_eq!(t.members.len(), 1);
        }
        i => panic!("{i:?}"),
    }
    match it.next() {
        Some(Item::Enum(e)) => {
            assert_eq!(e.members.len(), 2);
            assert!(e.members[1].value.is_some());
        }
        i => panic!("{i:?}"),
    }
    match it.next() {
        Some(Item::Declare(d)) => {
            assert!(d.ptr_safe);
            assert_eq!(d.kind, DeclareKind::Function);
            assert_eq!(d.lib, b"k");
            assert_eq!(d.alias.as_deref(), Some(&b"FA"[..]));
            assert_eq!(d.params.len(), 1);
            assert_eq!(d.params[0].pass_by, Some(PassBy::ByVal));
            assert!(d.return_type.is_some());
        }
        i => panic!("{i:?}"),
    }
    match it.next() {
        Some(Item::Proc(f)) => {
            assert_eq!(f.kind, ProcKind::Function);
            assert!(f.is_static);
            assert_eq!(f.visibility, Some(Visibility::Public));
            assert!(f.params[0].optional);
            assert!(f.params[1].param_array && f.params[1].is_array);
        }
        i => panic!("{i:?}"),
    }
    let kinds: Vec<ProcKind> = m.procs().map(|p| p.kind).collect();
    assert_eq!(
        kinds,
        [
            ProcKind::Function,
            ProcKind::PropertyGet,
            ProcKind::PropertyLet,
            ProcKind::PropertySet
        ]
    );
}

#[test]
fn member_attributes_and_header() {
    let p = clean(
        "VERSION 1.0 CLASS\nBEGIN\n  MultiUse = -1  'True\nEND\nAttribute VB_Name = \"C\"\nPublic Sub S()\nAttribute S.VB_Description = \"d\"\nEnd Sub\n",
    );
    let h = p.module.header.as_ref().unwrap();
    assert!(h.version.is_some() && h.block.is_some());
    assert_eq!(h.properties.len(), 1);
    assert_eq!(h.properties[0].name, Name::new("multiuse"));
    assert_eq!(h.properties[0].value, b"-1");
    let attrs: Vec<_> = p.module.procs().next().unwrap().attributes().collect();
    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].object.as_ref().unwrap().name, Name::new("s"));
    assert!(attrs[0].name.name.eq_ignore_case("vb_description"));
}

#[test]
fn conditional_compilation_markers() {
    let p = clean(
        "#If Win64 Then\nDim a\n#Else\nDim b\n#End If\nSub T()\n#If False Then\nx = 1\n#End If\nEnd Sub\n",
    );
    let kinds: Vec<_> = p
        .module
        .items
        .iter()
        .map(|i| match i {
            Item::Directive(d) => format!("{:?}={:?}", d.kind, d.value),
            Item::Inactive(_) => "inactive".to_string(),
            Item::Var(_) => "var".to_string(),
            Item::Proc(_) => "proc".to_string(),
            _ => "?".to_string(),
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "If=Some(true)",
            "var",
            "Else=None",
            "inactive",
            "EndIf=None",
            "proc"
        ]
    );
    let proc_ = p.module.procs().next().unwrap();
    assert!(
        proc_
            .body
            .stmts
            .iter()
            .any(|s| matches!(s.kind, StmtKind::Inactive))
    );
}

// ---- errors ------------------------------------------------------------------------

#[test]
fn error_nodes_become_error_items_with_diagnostics() {
    let p = parse("Sub A()\nx = )\ny = 2\nEnd Sub\n@@@\n");
    let codes: Vec<&str> = p.diagnostics.iter().map(|d| d.code.0).collect();
    assert!(codes.contains(&"AST001"), "{codes:?}");
    let proc_ = p.module.procs().next().unwrap();
    assert!(
        proc_
            .body
            .stmts
            .iter()
            .any(|s| matches!(s.kind, StmtKind::Error))
    );
    assert!(proc_
        .body
        .stmts
        .iter()
        .any(|s| matches!(&s.kind, StmtKind::Assign { value, .. } if matches!(value.kind, ExprKind::Literal(_)))));
    assert!(p.module.items.iter().any(|i| matches!(i, Item::Error(_))));
}

#[test]
fn incomplete_constructs_get_placeholders() {
    let p = parse("Sub A()\nFor i = 1\nNext\nx = \nEnd Sub\n");
    let codes: Vec<&str> = p.diagnostics.iter().map(|d| d.code.0).collect();
    assert!(codes.contains(&"AST002"), "{codes:?}");
}

// ---- fixtures -----------------------------------------------------------------------

const FIXTURES: &[(&str, &str)] = &[
    (
        "standard",
        include_str!("../../vba-syntax/tests/fixtures/standard_module.bas"),
    ),
    (
        "class",
        include_str!("../../vba-syntax/tests/fixtures/class_module.cls"),
    ),
    (
        "form",
        include_str!("../../vba-syntax/tests/fixtures/form_codebehind.frm"),
    ),
    (
        "api",
        include_str!("../../vba-syntax/tests/fixtures/api_declares.bas"),
    ),
];

#[test]
fn fixtures_lower_without_diagnostics() {
    for (name, text) in FIXTURES {
        let p = clean(text);
        assert!(!p.module.items.is_empty(), "{name}");
        assert_eq!(p.module.span.len() as usize, text.len());
    }
    let p = clean(FIXTURES[0].1);
    assert_eq!(p.module.vb_name(), Some(Name::new("utilities")));
    assert_eq!(p.module.procs().count(), 10);
    let p = clean(FIXTURES[1].1);
    assert_eq!(p.module.vb_name(), Some(Name::new("Account")));
    assert_eq!(p.module.procs().count(), 12);
    let balance = p
        .module
        .procs()
        .find(|p| p.name.name == Name::new("balance"))
        .unwrap();
    assert_eq!(balance.kind, ProcKind::PropertyGet);
    assert_eq!(balance.attributes().count(), 2);
    let p = clean(FIXTURES[2].1);
    assert!(p.module.header.as_ref().unwrap().block.is_some());
    let p = clean(FIXTURES[3].1);
    let declares = p
        .module
        .items
        .iter()
        .filter(|i| matches!(i, Item::Declare(_)))
        .count();
    assert_eq!(declares, 7);
}
