' INV: vbal.5.3.1.5, decl.byref-byval
' SOURCE: spec ms-vbal 5.3.1.5 omitted mechanism means ByRef; ByVal gets its own variable
' EXPECT: r=10 v=1
Sub S(a As Long, ByVal b As Long, ByRef c As Long)
    a = a + 9
    b = b + 9
    c = c + 9
End Sub
Public Sub Main()
    Dim r As Long, v As Long, q As Long
    r = 1: v = 1: q = 1
    S r, v, q
    Debug.Print "r=" & CStr(r) & " v=" & CStr(v)
End Sub
