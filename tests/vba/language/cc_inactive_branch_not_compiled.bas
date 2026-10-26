' INV: cc.if
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: ok
#If False Then
this is not valid VBA at all (((
#End If
Public Sub Main()
    Debug.Print "ok"
End Sub
