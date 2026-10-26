' INV: vbal.5.2.3.1.4
' SOURCE: spec ms-vbal 5.2.3.1.4 fixed-length-string-spec String*n
' EXPECT: 5
' EXPECT: 4
Private Const L As Long = 4
Dim s As String * 5
Dim t As String * L
Public Sub Main()
    Debug.Print CStr(Len(s))
    Debug.Print CStr(Len(t))
End Sub
