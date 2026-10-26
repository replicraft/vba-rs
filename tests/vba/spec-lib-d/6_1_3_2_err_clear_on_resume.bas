' INV: vbal.6.1.3.2.1.1
' SOURCE: spec ms-vbal 6.1.3.2.1.1 Err.Clear is called automatically by a Resume statement
' EXPECT: handler=5
' EXPECT: resumed=0
Public Sub Main()
    On Error GoTo h
    Err.Raise 5
    Debug.Print "resumed=" & CStr(Err.Number)
    Exit Sub
h:
    Debug.Print "handler=" & CStr(Err.Number)
    Resume Next
End Sub
