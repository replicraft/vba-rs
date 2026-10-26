' INV: cc.if
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: vba7
Public Sub Main()
#If VBA7 Then
    Debug.Print "vba7"
#Else
    Debug.Print "legacy"
#End If
End Sub
