' INV: func.join, func.split
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  3 a|b||c
Public Sub Main()
    Dim a() As String
    a = Split("a,b,,c", ",")
    Debug.Print UBound(a); Join(a, "|")
End Sub
