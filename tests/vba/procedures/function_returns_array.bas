' INV: stmt.function
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  2  6 
' EXPECT:  5 
Function Mk() As Long()
    Dim a(1 To 2) As Long
    a(1) = 5: a(2) = 6
    Mk = a
End Function
Public Sub Main()
    Dim r() As Long
    r = Mk()
    Debug.Print UBound(r); r(2)
    Debug.Print Mk()(1)
End Sub
