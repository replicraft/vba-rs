' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 after normal completion the counter holds the incremented value; Exit For keeps it
' EXPECT: after=4
' EXPECT: exited=2
' EXPECT: none=5
Public Sub Main()
    Dim i As Long
    For i = 1 To 3
    Next i
    Debug.Print "after=" & CStr(i)
    For i = 1 To 3
        If i = 2 Then Exit For
    Next i
    Debug.Print "exited=" & CStr(i)
    For i = 5 To 1
        Debug.Print "never"
    Next i
    Debug.Print "none=" & CStr(i)
End Sub
