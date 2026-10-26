' INV: decl.array-bounds, decl.byref-byval
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  42 
Sub Fill(a() As Long)
    a(1) = 42
End Sub
Public Sub Main()
    Dim x(1 To 2) As Long
    Fill x
    Debug.Print x(1)
End Sub
