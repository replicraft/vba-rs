' INV: func.ubound, func.split
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  2 
Public Sub Main()
    Debug.Print UBound(Split(",,", ","))
End Sub
