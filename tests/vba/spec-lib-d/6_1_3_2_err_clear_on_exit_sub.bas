' INV: vbal.6.1.3.2.1.1
' SOURCE: spec ms-vbal 6.1.3.2.1.1 Err.Clear is called automatically by Exit Sub
' EXPECT: inside=5
' EXPECT: after=0
Private Sub Callee()
    On Error GoTo h
    Err.Raise 5
    Exit Sub
h:
    Debug.Print "inside=" & CStr(Err.Number)
    Exit Sub
End Sub
Public Sub Main()
    Callee
    Debug.Print "after=" & CStr(Err.Number)
End Sub
