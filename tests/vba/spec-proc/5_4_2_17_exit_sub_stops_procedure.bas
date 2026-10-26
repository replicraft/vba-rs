' INV: vbal.5.4.2.17, stmt.exit
' SOURCE: spec ms-vbal 5.4.2.17 Exit Sub completes the procedure from inside nested constructs
' EXPECT: 1
' EXPECT: back
Sub S()
    Dim i As Long
    For i = 1 To 5
        Do
            Debug.Print CStr(i)
            If i = 1 Then Exit Sub
        Loop While False
    Next i
    Debug.Print "never"
End Sub
Public Sub Main()
    S
    Debug.Print "back"
End Sub
