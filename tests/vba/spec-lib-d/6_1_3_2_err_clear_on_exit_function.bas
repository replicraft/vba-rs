' INV: vbal.6.1.3.2.1.1
' SOURCE: spec ms-vbal 6.1.3.2.1.1 Err.Clear is called automatically by Exit Function
' EXPECT: inside=5
' EXPECT: after=0
Private Function Callee() As Long
    On Error GoTo h
    Err.Raise 5
    Exit Function
h:
    Debug.Print "inside=" & CStr(Err.Number)
    Exit Function
End Function
Public Sub Main()
    Dim x As Long
    x = Callee()
    Debug.Print "after=" & CStr(Err.Number)
End Sub
