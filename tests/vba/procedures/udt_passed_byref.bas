' INV: stmt.sub, stmt.type, decl.byref-byval
' SOURCE: spec ms-vbal 5.3.1.11 ByRef binding of a variable argument
' EXPECT:  2 
Type P
    X As Long
End Type
Sub Bump(ByRef q As P)
    q.X = q.X + 1
End Sub
Public Sub Main()
    Dim p As P
    Bump p
    Bump p
    Debug.Print p.X
End Sub
