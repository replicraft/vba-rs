' INV: cc.if
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: win64
#If Win64 Then
Public Sub Main()
    Debug.Print "win64"
End Sub
#Else
Public Sub Main()
    Debug.Print "other"
End Sub
#End If
