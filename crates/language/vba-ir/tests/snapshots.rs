//! Snapshot tests: one small procedure per statement kind, dumped as text.

mod common;

use common::dump;
use expect_test::{Expect, expect};

fn check(src: &str, name: &str, expect: Expect) {
    expect.assert_eq(&dump(src, name));
}

#[test]
fn assign_let_scalar() {
    check(
        "Sub P()\nDim i As Integer, l As Long, d As Double\nl = i + 1\nd = l / 2\ni = l\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 i : Integer (local)
              slot s1 l : Long (local)
              slot s2 d : Double (local)
              b0:
                r0 = load.local s0:i
                r1 = add.typed:Integer r0, 1%
                r2 = convert:Long r1
                store.local s1:l <- r2 [let]
                r3 = load.local s1:l
                r4 = div.typed:Double r3, 2%
                store.local s2:d <- r4 [let]
                r5 = load.local s1:l
                r6 = convert:Integer r5
                store.local s0:i <- r6 [let]
                return
        "#]],
    );
}

#[test]
fn assign_set_object() {
    check(
        "Sub P()\nDim c As Collection, o As Object\nSet c = New Collection\nSet o = c\nSet o = Nothing\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 c : VBA.Collection (local)
              slot s1 o : Object (local)
              b0:
                r0 = new lib VBA.Collection
                store.local s0:c <- r0 [set]
                r1 = load.local s0:c
                store.local s1:o <- r1 [set]
                store.local s1:o <- Nothing [set]
                return
        "#]],
    );
}

#[test]
fn function_result_and_recursion() {
    check(
        "Function Fact(ByVal n As Long) As Long\nIf n <= 1 Then\nFact = 1\nElse\nFact = n * Fact(n - 1)\nEnd If\nEnd Function\n",
        "Fact",
        expect![[r#"
            func M.Fact
              slot s0 n : Long (param 0)
              slot s1 Fact : Long (result)
              b0:
                r0 = load.local s0:n
                r1 = le[bin].typed:Boolean r0, 1%
                branch r1 ? b2 : b1
              b1:
                r2 = load.local s0:n
                r3 = load.local s0:n
                r4 = sub.typed:Long r3, 1%
                r5 = call M.Fact(r4)
                r6 = mul.typed:Long r2, r5
                store.local s1:Fact <- r6 [let]
                jump b3
              b2:
                store.local s1:Fact <- 1& [let]
                jump b3
              b3:
                return
        "#]],
    );
}

#[test]
fn if_elseif_else() {
    check(
        "Sub P(ByVal x As Long)\nDim y As Long\nIf x < 0 Then\ny = -1\nElseIf x = 0 Then\ny = 0\nElse\ny = 1\nEnd If\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 x : Long (param 0)
              slot s1 y : Long (local)
              b0:
                r0 = load.local s0:x
                r1 = lt[bin].typed:Boolean r0, 0%
                branch r1 ? b4 : b1
              b1:
                r2 = load.local s0:x
                r3 = eq[bin].typed:Boolean r2, 0%
                branch r3 ? b3 : b2
              b2:
                store.local s1:y <- 1& [let]
                jump b5
              b3:
                store.local s1:y <- 0& [let]
                jump b5
              b4:
                store.local s1:y <- -1& [let]
                jump b5
              b5:
                return
        "#]],
    );
}

#[test]
fn if_single_line() {
    check(
        "Sub P(ByVal x As Long)\nDim y As Long\nIf x Then y = 1 Else y = 2\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 x : Long (param 0)
              slot s1 y : Long (local)
              b0:
                r0 = load.local s0:x
                r1 = convert:Boolean r0
                branch r1 ? b2 : b1
              b1:
                store.local s1:y <- 2& [let]
                jump b3
              b2:
                store.local s1:y <- 1& [let]
                jump b3
              b3:
                return
        "#]],
    );
}

#[test]
fn select_case() {
    check(
        "Sub P(ByVal x As Long)\nDim y As Long\nSelect Case x\nCase 1, 2\ny = 1\nCase 3 To 5\ny = 2\nCase Is > 10\ny = 3\nCase Else\ny = 4\nEnd Select\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 x : Long (param 0)
              slot s1 y : Long (local)
              b0:
                r0 = load.local s0:x
                r1 = eq[bin].typed:Boolean r0, 1%
                branch r1 ? b8 : b1
              b1:
                r2 = eq[bin].typed:Boolean r0, 2%
                branch r2 ? b8 : b2
              b2:
                r3 = ge[bin].typed:Boolean r0, 3%
                branch r3 ? b3 : b4
              b3:
                r4 = le[bin].typed:Boolean r0, 5%
                branch r4 ? b7 : b4
              b4:
                r5 = gt[bin].typed:Boolean r0, 10%
                branch r5 ? b6 : b5
              b5:
                store.local s1:y <- 4& [let]
                jump b9
              b6:
                store.local s1:y <- 3& [let]
                jump b9
              b7:
                store.local s1:y <- 2& [let]
                jump b9
              b8:
                store.local s1:y <- 1& [let]
                jump b9
              b9:
                return
        "#]],
    );
}

#[test]
fn select_case_text_compare() {
    check(
        "Option Compare Text\nSub P(ByVal s As String)\nSelect Case s\nCase \"a\"\nDebug.Print 1\nEnd Select\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 s : String (param 0)
              b0:
                r0 = load.local s0:s
                r1 = eq[text].typed:Boolean r0, "a"
                branch r1 ? b1 : b2
              b1:
                debug.print 1%
                jump b2
              b2:
                return
        "#]],
    );
}

#[test]
fn for_next_step() {
    check(
        "Sub P()\nDim i As Long, s As Long\nFor i = 10 To 1 Step -2\ns = s + i\nNext i\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 i : Long (local)
              slot s1 s : Long (local)
              b0:
                r0 = copy True
                store.local s0:i <- 10& [let]
                jump b1
              b1:
                r1 = load.local s0:i
                r2 = for.test r1, 1&, -2&
                branch r2 ? b3 : b2
              b2:
                return
              b3:
                r3 = load.local s1:s
                r4 = load.local s0:i
                r5 = add.typed:Long r3, r4
                store.local s1:s <- r5 [let]
                r6 = load.local s0:i
                r7 = add.typed:Long r6, -2& guard r0
                store.local s0:i <- r7 [let]
                jump b1
        "#]],
    );
}

#[test]
fn for_next_variant_counter() {
    check(
        "Sub P()\nDim v As Variant\nFor v = 1 To 3\nDebug.Print v\nNext\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 v : Variant (local)
              b0:
                r0 = copy True
                store.local s0:v <- 1% [let]
                jump b1
              b1:
                r1 = load.local s0:v
                r2 = for.test r1, 3%, 1%
                branch r2 ? b3 : b2
              b2:
                return
              b3:
                r3 = load.local s0:v
                debug.print r3
                r4 = load.local s0:v
                r5 = add.variant:Variant r4, 1% guard r0
                store.local s0:v <- r5 [let]
                jump b1
        "#]],
    );
}

#[test]
fn for_each_array() {
    check(
        "Sub P()\nDim a(1 To 3) As Long, v As Variant\nFor Each v In a\nDebug.Print v\nNext\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 a : Long() (local)
              slot s1 v : Variant (local)
              b0:
                r0 = ref s0:a
                dim.fixed r0 as Long (1 To 3)
                r1 = load.local s0:a
                r2 = iter.init r1
                jump b1
              b1:
                foreach.next r2 -> r3 ? b3 : b2
              b2:
                return
              b3:
                store.local s1:v <- r3 [let]
                r4 = load.local s1:v
                debug.print r4
                jump b1
        "#]],
    );
}

#[test]
fn for_each_collection_object() {
    check(
        "Sub P(c As Collection)\nDim o As Object\nFor Each o In c\nDebug.Print o.Name\nNext o\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 c : VBA.Collection (param 0)
              slot s1 o : Object (local)
              b0:
                r0 = load.local s0:c
                r1 = iter.init r0
                jump b1
              b1:
                foreach.next r1 -> r2 ? b3 : b2
              b2:
                return
              b3:
                store.local s1:o <- r2 [set]
                r3 = load.local s1:o
                r4 = invoke.get r3.Name()
                debug.print r4
                jump b1
        "#]],
    );
}

#[test]
fn do_while_pre() {
    check(
        "Sub P()\nDim i As Long\nDo While i < 3\ni = i + 1\nLoop\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 i : Long (local)
              b0:
                jump b1
              b1:
                r0 = load.local s0:i
                r1 = lt[bin].typed:Boolean r0, 3%
                branch r1 ? b3 : b2
              b2:
                return
              b3:
                r2 = load.local s0:i
                r3 = add.typed:Long r2, 1%
                store.local s0:i <- r3 [let]
                jump b1
        "#]],
    );
}

#[test]
fn do_until_post() {
    check(
        "Sub P()\nDim i As Long\nDo\ni = i + 1\nLoop Until i >= 3\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 i : Long (local)
              b0:
                jump b1
              b1:
                r0 = load.local s0:i
                r1 = add.typed:Long r0, 1%
                store.local s0:i <- r1 [let]
                r2 = load.local s0:i
                r3 = ge[bin].typed:Boolean r2, 3%
                branch r3 ? b2 : b1
              b2:
                return
        "#]],
    );
}

#[test]
fn do_forever_exit() {
    check(
        "Sub P()\nDim i As Long\nDo\ni = i + 1\nIf i > 3 Then Exit Do\nLoop\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 i : Long (local)
              b0:
                jump b1
              b1:
                r0 = load.local s0:i
                r1 = add.typed:Long r0, 1%
                store.local s0:i <- r1 [let]
                r2 = load.local s0:i
                r3 = gt[bin].typed:Boolean r2, 3%
                branch r3 ? b2 : b1
              b2:
                return
        "#]],
    );
}

#[test]
fn while_wend() {
    check(
        "Sub P()\nDim i As Long\nWhile i < 3\ni = i + 1\nWend\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 i : Long (local)
              b0:
                jump b1
              b1:
                r0 = load.local s0:i
                r1 = lt[bin].typed:Boolean r0, 3%
                branch r1 ? b3 : b2
              b2:
                return
              b3:
                r2 = load.local s0:i
                r3 = add.typed:Long r2, 1%
                store.local s0:i <- r3 [let]
                jump b1
        "#]],
    );
}

#[test]
fn exit_for_inside_do() {
    check(
        "Sub P()\nDim i As Long\nFor i = 1 To 3\nDo\nExit Do\nLoop\nIf i = 2 Then Exit For\nNext\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 i : Long (local)
              b0:
                r0 = copy True
                store.local s0:i <- 1& [let]
                jump b1
              b1:
                r1 = load.local s0:i
                r2 = for.test r1, 3&, 1%
                branch r2 ? b2 : b4
              b2:
                r3 = load.local s0:i
                r4 = eq[bin].typed:Boolean r3, 2%
                branch r4 ? b4 : b3
              b3:
                r5 = load.local s0:i
                r6 = add.typed:Long r5, 1% guard r0
                store.local s0:i <- r6 [let]
                jump b1
              b4:
                return
        "#]],
    );
}

#[test]
fn with_object() {
    check(
        "Sub P(c As Collection)\nWith c\n.Add 1\nDebug.Print .Count\nEnd With\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 c : VBA.Collection (param 0)
              slot s1  : VBA.Collection (temp)
              b0:
                r0 = load.local s0:c
                store.local s1 <- r0 [set]
                r1 = load.local s1
                invoke.get r1.Add(1%, missing, missing, missing) ; VBA.Collection
                r2 = load.local s1
                r3 = invoke.get r2.Count() ; VBA.Collection
                debug.print r3
                store.local s1 <- Nothing [set]
                return
        "#]],
    );
}

#[test]
fn with_udt() {
    check(
        "Type T\nx As Long\nEnd Type\nSub P()\nDim u As T\nWith u\n.x = 1\nDebug.Print .x\nEnd With\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 u : T (local)
              b0:
                r0 = ref s0:u
                r1 = ref r0.T.x
                store.ref r1 <- 1& [let]
                r2 = load.local s0:u
                r3 = udt.field r2.T.x
                debug.print r3
                return
        "#]],
    );
}

#[test]
fn goto_and_labels() {
    check(
        "Sub P()\nDim i As Long\ntop:\ni = i + 1\nIf i < 3 Then GoTo top\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 i : Long (local)
              b0:
                jump b1
              b1:
                r0 = load.local s0:i
                r1 = add.typed:Long r0, 1%
                store.local s0:i <- r1 [let]
                r2 = load.local s0:i
                r3 = lt[bin].typed:Boolean r2, 3%
                branch r3 ? b1 : b2
              b2:
                return
        "#]],
    );
}

#[test]
fn line_numbers() {
    check(
        "Sub P()\n10 Debug.Print 1\n20 Debug.Print 2\nGoTo 10\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              b0:
                jump b1
              b1:
                set.line 10
                debug.print 1%
                set.line 20
                debug.print 2%
                jump b1
        "#]],
    );
}

#[test]
fn gosub_return() {
    check(
        "Sub P()\nGoSub sub1\nExit Sub\nsub1:\nDebug.Print 1\nReturn\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              b0:
                gosub b2 ret b1
              b1:
                return
              b2:
                debug.print 1%
                return.gosub
        "#]],
    );
}

#[test]
fn on_goto() {
    check(
        "Sub P(ByVal n As Long)\nOn n GoTo a, b\nDebug.Print 0\na:\nDebug.Print 1\nb:\nDebug.Print 2\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 n : Long (param 0)
              b0:
                r0 = load.local s0:n
                switch r0 [b2, b3] default b1
              b1:
                debug.print 0%
                jump b2
              b2:
                debug.print 1%
                jump b3
              b3:
                debug.print 2%
                return
        "#]],
    );
}

#[test]
fn on_gosub() {
    check(
        "Sub P(ByVal n As Long)\nOn n GoSub a\nExit Sub\na:\nDebug.Print 1\nReturn\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 n : Long (param 0)
              b0:
                r0 = load.local s0:n
                switch r0 [b1] default b2
              b1:
                gosub b3 ret b2
              b2:
                return
              b3:
                debug.print 1%
                return.gosub
        "#]],
    );
}

#[test]
fn on_error_goto_resume_next() {
    check(
        "Sub P()\nOn Error GoTo h\nDebug.Print 1 / 0\nExit Sub\nh:\nDebug.Print Err.Number\nResume Next\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              b0:
                on.error goto b1
                r0 = div.typed:Double 1%, 0%
                debug.print r0
                return
              b1:
                r1 = lib.global Err
                r2 = invoke.get r1.Number() ; VBA.ErrObject
                debug.print r2
                resume.next
        "#]],
    );
}

#[test]
fn on_error_resume_next_and_reset() {
    check(
        "Sub P()\nOn Error Resume Next\nDebug.Print 1\nOn Error GoTo 0\nOn Error GoTo -1\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              b0:
                on.error resume.next
                debug.print 1%
                on.error propagate
                clear.error
                return
        "#]],
    );
}

#[test]
fn resume_forms() {
    check(
        "Sub P()\nOn Error GoTo h\nDebug.Print 1\nExit Sub\nh:\nResume\nResume Next\nResume done\ndone:\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              b0:
                on.error goto b1
                debug.print 1%
                return
              b1:
                resume
        "#]],
    );
}

#[test]
fn call_by_ref_val_optional() {
    check(
        "Sub Q(ByRef a As Long, ByVal b As Long, Optional c As String)\nEnd Sub\nSub P()\nDim x As Long, arr(1 To 2) As Long\nQ x, 1\nQ arr(1), x + 1, \"s\"\nCall Q((x), 2)\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 x : Long (local)
              slot s1 arr : Long() (local)
              b0:
                r0 = ref s1:arr
                dim.fixed r0 as Long (1 To 2)
                r1 = ref s0:x
                call M.Q(ref r1, 1&, missing)
                r2 = ref s1:arr
                r3 = ref r2[1%]
                r4 = load.local s0:x
                r5 = add.typed:Long r4, 1%
                call M.Q(ref r3, r5, "s")
                r6 = load.local s0:x
                call M.Q(r6, 2&, missing)
                return
        "#]],
    );
}

#[test]
fn call_named_and_paramarray() {
    check(
        "Sub Q(ByVal a As Long, Optional b As Long, ParamArray rest() As Variant)\nEnd Sub\nSub P()\nQ b:=2, a:=1\nQ 1, 2, 3, \"x\"\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              b0:
                call M.Q(1&, 2&)
                call M.Q(1&, 2&, 3%, "x")
                return
        "#]],
    );
}

#[test]
fn declare_call() {
    check(
        "Private Declare PtrSafe Function GetTickCount Lib \"kernel32\" () As Long\nSub P()\nDim t As Long\nt = GetTickCount()\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 t : Long (local)
              b0:
                r0 = call declare GetTickCount()
                store.local s0:t <- r0 [let]
                return
        "#]],
    );
}

#[test]
fn intrinsic_calls() {
    check(
        "Sub P(s As String)\nDim n As Long, t As String\nn = Len(s)\nt = Left$(s, 2) & UCase(s)\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 s : String (param 0)
              slot s1 n : Long (local)
              slot s2 t : String (local)
              b0:
                r0 = load.local s0:s
                r1 = convert:Variant r0
                r2 = call intrinsic Len(r1)
                r3 = convert:Long r2
                store.local s1:n <- r3 [let]
                r4 = load.local s0:s
                r5 = convert:Variant r4
                r6 = call intrinsic Left$(r5, 2&)
                r7 = load.local s0:s
                r8 = convert:Variant r7
                r9 = call intrinsic UCase(r8)
                r10 = concat.variant:Variant r6, r9
                r11 = convert:String r10
                store.local s2:t <- r11 [let]
                return
        "#]],
    );
}

#[test]
fn late_bound_variant_and_object() {
    check(
        "Sub P(o As Object, v As Variant)\nDim x As Variant\nx = o.Name\no.Name = \"a\"\nSet o.Parent = v\nv.Run 1, 2\nx = v(3)\nx = o!key\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 o : Object (param 0)
              slot s1 v : Variant (param 1)
              slot s2 x : Variant (local)
              b0:
                r0 = load.local s0:o
                r1 = invoke.get r0.Name()
                store.local s2:x <- r1 [let]
                r2 = load.local s0:o
                invoke.put r2.Name("a")
                r3 = load.local s1:v
                r4 = call intrinsic SetCheckVariant(r3)
                r5 = load.local s0:o
                invoke.putref r5.Parent(r4)
                r6 = load.local s1:v
                invoke.get r6.Run(1%, 2%)
                r7 = load.local s1:v
                r8 = invoke.get r7.<default>(3%)
                store.local s2:x <- r8 [let]
                r9 = load.local s0:o
                r10 = invoke.get r9.<default>("key")
                store.local s2:x <- r10 [let]
                return
        "#]],
    );
}

#[test]
fn library_member_early() {
    check(
        "Sub P()\nDim d As New Scripting.Dictionary\nDim n As Long\nd.Add \"k\", 1\nn = d.Count\nn = d(\"k\")\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 d : Scripting.Dictionary (local)
              slot s1 n : Long (local)
              b0:
                r0 = ref s0:d
                ensure.new r0 lib Scripting.Dictionary
                r1 = load.local s0:d
                invoke.get r1.Add("k", 1%) ; Scripting.Dictionary
                r2 = ref s0:d
                ensure.new r2 lib Scripting.Dictionary
                r3 = load.local s0:d
                r4 = invoke.get r3.Count() ; Scripting.Dictionary
                store.local s1:n <- r4 [let]
                r5 = ref s0:d
                ensure.new r5 lib Scripting.Dictionary
                r6 = load.local s0:d
                r7 = invoke.get r6.Item("k") ; Scripting.Dictionary
                r8 = convert:Long r7
                store.local s1:n <- r8 [let]
                return
        "#]],
    );
}

#[test]
fn auto_new_variable() {
    check(
        "Sub P()\nDim c As New Collection\nc.Add 1\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 c : VBA.Collection (local)
              b0:
                r0 = ref s0:c
                ensure.new r0 lib VBA.Collection
                r1 = load.local s0:c
                invoke.get r1.Add(1%, missing, missing, missing) ; VBA.Collection
                return
        "#]],
    );
}

#[test]
fn auto_new_variable_passed_by_reference_is_instantiated() {
    check(
        "Sub Q(c As Collection)\nEnd Sub\nSub P()\nDim c As New Collection\nQ c\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 c : VBA.Collection (local)
              b0:
                r0 = ref s0:c
                ensure.new r0 lib VBA.Collection
                call M.Q(ref r0)
                return
        "#]],
    );
}

#[test]
fn redim_erase_fixed_array() {
    check(
        "Sub P()\nDim f(1 To 3, 0 To 1) As Long, d() As Long\nReDim d(5)\nReDim Preserve d(1 To 8)\nErase d, f\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 f : Long(,) (local)
              slot s1 d : Long() (local)
              b0:
                r0 = ref s0:f
                dim.fixed r0 as Long (1 To 3, 0 To 1)
                r1 = ref s1:d
                redim r1 as Long (5%)
                r2 = ref s1:d
                redim.preserve r2 as Long (1% To 8%)
                r3 = ref s1:d
                erase r3
                r4 = ref s0:f
                erase r4
                return
        "#]],
    );
}

#[test]
fn udt_fields_and_array_elements() {
    check(
        "Type T\nx As Long\nEnd Type\nSub P()\nDim u As T, a(2) As T, n As Long\nu.x = 1\nn = u.x\na(1).x = n\nn = a(2).x\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 u : T (local)
              slot s1 a : T() (local)
              slot s2 n : Long (local)
              b0:
                r0 = ref s1:a
                dim.fixed r0 as T (0 To 2)
                r1 = ref s0:u
                r2 = ref r1.T.x
                store.ref r2 <- 1& [let]
                r3 = load.local s0:u
                r4 = udt.field r3.T.x
                store.local s2:n <- r4 [let]
                r5 = load.local s2:n
                r6 = ref s1:a
                r7 = ref r6[1%]
                r8 = ref r7.T.x
                store.ref r8 <- r5 [let]
                r9 = load.local s1:a
                r10 = index r9[2%]
                r11 = udt.field r10.T.x
                store.local s2:n <- r11 [let]
                return
        "#]],
    );
}

#[test]
fn file_io() {
    check(
        "Sub P()\nDim s As String, n As Long\nOpen \"f.txt\" For Output As #1\nPrint #1, \"a\"; 1, 2\nWrite #1, s\nClose #1\nOpen \"f.txt\" For Input As #2\nLine Input #2, s\nInput #2, n\nClose\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 s : String (local)
              slot s1 n : Long (local)
              b0:
                io.open "f.txt" for Output access None lock None as 1% len -
                io.print 1% : "a" ; 1% , 2%
                r0 = load.local s0:s
                io.write 1% : r0
                io.close 1%
                io.open "f.txt" for Input access None lock None as 2% len -
                r1 = ref s0:s
                io.lineinput 2% -> r1
                r2 = ref s1:n
                io.input 2% -> r2
                io.close.all
                return
        "#]],
    );
}

#[test]
fn debug_print() {
    check(
        "Sub P()\nDebug.Print \"a\"; 1, 2\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              b0:
                debug.print "a" ; 1% , 2%
                return
        "#]],
    );
}

#[test]
fn operators_and_comparisons() {
    check(
        "Sub P(ByVal a As Long, ByVal s As String, v As Variant)\nDim b As Boolean\nb = a > 1 And s Like \"x*\" Or Not b\nb = s = \"a\"\nb = v = 1\nb = a = \"1\"\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 a : Long (param 0)
              slot s1 s : String (param 1)
              slot s2 v : Variant (param 2)
              slot s3 b : Boolean (local)
              b0:
                r0 = load.local s0:a
                r1 = gt[bin].typed:Boolean r0, 1%
                r2 = load.local s1:s
                r3 = like[bin].typed:Boolean r2, "x*"
                r4 = and.typed:Boolean r1, r3
                r5 = load.local s3:b
                r6 = not.typed:Boolean r5
                r7 = or.typed:Boolean r4, r6
                store.local s3:b <- r7 [let]
                r8 = load.local s1:s
                r9 = eq[bin].typed:Boolean r8, "a"
                store.local s3:b <- r9 [let]
                r10 = load.local s2:v
                r11 = eq[bin].variant:Variant r10, 1%
                r12 = convert:Boolean r11
                store.local s3:b <- r12 [let]
                r13 = load.local s0:a
                r14 = eq[bin+coerce].typed:Boolean r13, "1"
                store.local s3:b <- r14 [let]
                return
        "#]],
    );
}

#[test]
fn is_and_typeof() {
    check(
        "Sub P(o As Object, c As Collection)\nDim b As Boolean\nb = o Is c\nb = TypeOf o Is Collection\nb = o Is Nothing\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 o : Object (param 0)
              slot s1 c : VBA.Collection (param 1)
              slot s2 b : Boolean (local)
              b0:
                r0 = load.local s0:o
                r1 = load.local s1:c
                r2 = is.typed:Boolean r0, r1
                store.local s2:b <- r2 [let]
                r3 = load.local s0:o
                r4 = typeof.is r3 VBA.Collection
                store.local s2:b <- r4 [let]
                r5 = load.local s0:o
                r6 = is.typed:Boolean r5, Nothing
                store.local s2:b <- r6 [let]
                return
        "#]],
    );
}

#[test]
fn static_and_module_variables() {
    check(
        "Dim counter As Long\nSub P()\nStatic n As Long\nn = n + 1\ncounter = counter + n\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              b0:
                r0 = load.global g1:M.n
                r1 = add.typed:Long r0, 1%
                store.global g1:M.n <- r1 [let]
                r2 = load.global g0:M.counter
                r3 = load.global g1:M.n
                r4 = add.typed:Long r2, r3
                store.global g0:M.counter <- r4 [let]
                return
        "#]],
    );
}

#[test]
fn end_and_stop() {
    check(
        "Sub P()\nStop\nEnd\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              b0:
                stop
                end
        "#]],
    );
}

#[test]
fn exit_sub_and_function() {
    check(
        "Function F() As Long\nF = 1\nExit Function\nEnd Function\n",
        "F",
        expect![[r#"
            func M.F
              slot s0 F : Long (result)
              b0:
                store.local s0:F <- 1& [let]
                return
        "#]],
    );
}

#[test]
fn address_of() {
    check(
        "Sub Cb()\nEnd Sub\nSub P()\nDim p As LongPtr\np = AddressOf Cb\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 p : LongLong (local)
              b0:
                r0 = addressof M.Cb
                store.local s0:p <- r0 [let]
                return
        "#]],
    );
}

#[test]
fn parenthesised_object_default_member() {
    check(
        "Sub P(c As Collection)\nDim v As Variant\nv = (c)\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 c : VBA.Collection (param 0)
              slot s1 v : Variant (local)
              b0:
                r0 = load.local s0:c
                r1 = invoke.get r0.Item() ; VBA.Collection
                store.local s1:v <- r1 [let]
                return
        "#]],
    );
}

#[test]
fn mid_statement() {
    check(
        "Sub P()\nDim s As String\nMid(s, 2, 1) = \"x\"\nMid$(s, 3) = \"yz\"\nEnd Sub\n",
        "P",
        expect![[r#"
            func M.P
              slot s0 s : String (local)
              b0:
                r0 = ref s0:s
                call intrinsic MidStatement(ref r0, 2%, 1%, "x")
                r1 = ref s0:s
                call intrinsic MidStatement(ref r1, 3%, missing, "yz")
                return
        "#]],
    );
}
