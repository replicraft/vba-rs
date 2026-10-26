' INV: func.vartype, type.object
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  9 
Public Sub Main()
    Dim c As New Collection, o As Object
    Set o = c
    Debug.Print VarType(o)
End Sub
