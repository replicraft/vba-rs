' INV: vbal.5.3.1.11, decl.byref-byval
' SOURCE: spec ms-vbal 5.3.1.11 ByRef variable arguments are reference bindings to the same variable
' EXPECT: b=5
Sub S(a As Long, b As Long)
    a = 5
    Debug.Print "b=" & CStr(b)
End Sub
Public Sub Main()
    Dim x As Long
    S x, x
End Sub
