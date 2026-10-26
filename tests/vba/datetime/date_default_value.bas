' INV: type.date
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  0 
' EXPECT:  1899 
Public Sub Main()
    Dim d As Date
    Debug.Print CDbl(d)
    Debug.Print Year(d)
End Sub
