' INV: type.string
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  3 
' EXPECT: False
Public Sub Main()
    Dim s As String
    s = "a" & Chr(0) & "b"
    Debug.Print Len(s)
    Debug.Print s = "a"
End Sub
