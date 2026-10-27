' INV: expr.named-arg
' SOURCE: unverified the implementation evaluates arguments in parameter order (a, b, c); whether real VBA evaluates in the order written (c, a, b) is an open question, see docs/language/spec-coverage.md
' METHOD: by hand; the output is the implementation's own behavior and is the program the oracle run must settle
' EXPECT: eval a
' EXPECT: eval b
' EXPECT: eval c
' EXPECT: abc
Private Function Trace(ByVal s As String) As String
    Debug.Print "eval " & s
    Trace = s
End Function
Private Sub Show(ByVal a As String, ByVal b As String, ByVal c As String)
    Debug.Print a & b & c
End Sub
Public Sub Main()
    Show c:=Trace("c"), a:=Trace("a"), b:=Trace("b")
End Sub
