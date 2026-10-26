' INV: type.boolean
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: -2 
' EXPECT:  1 
' EXPECT: -5 
Public Sub Main()
    Debug.Print True + True
    Debug.Print -True
    Debug.Print True * 5
End Sub
