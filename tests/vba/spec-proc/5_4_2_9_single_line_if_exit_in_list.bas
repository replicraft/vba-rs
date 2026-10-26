' INV: vbal.5.4.2.9, stmt.if-then-else
' SOURCE: spec ms-vbal 5.4.2.9 a statement in the list may transfer control out of it
' EXPECT: 1
Public Sub Main()
    Dim i As Long
    For i = 1 To 3
        Debug.Print CStr(i): If i = 1 Then Exit For: Debug.Print "no"
    Next i
End Sub
