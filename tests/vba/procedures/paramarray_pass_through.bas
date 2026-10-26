' INV: decl.paramarray
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  0  1  3  1 
Function Count(ParamArray v() As Variant) As Long
    Count = UBound(v) - LBound(v) + 1
End Function
Public Sub Main()
    Debug.Print Count(); Count(1); Count(1, 2, 3); Count(Array(1, 2))
End Sub
