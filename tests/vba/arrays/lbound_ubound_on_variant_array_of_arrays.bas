' INV: func.lbound, func.ubound
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  1  2  0 
Public Sub Main()
    Dim v As Variant
    v = Array(Array(1, 2, 3), Array(4))
    Debug.Print UBound(v); UBound(v(0)); UBound(v(1))
End Sub
