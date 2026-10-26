' INV: stmt.sub, decl.byref-byval
' SOURCE: spec ms-vbal 5.3.1.11 ByRef binding of a variable argument
' EXPECT:  5050 
Sub Walk(ByVal n As Long, ByRef acc As Long)
    If n = 0 Then Exit Sub
    acc = acc + n
    Walk n - 1, acc
End Sub
Public Sub Main()
    Dim t As Long
    Walk 100, t
    Debug.Print t
End Sub
