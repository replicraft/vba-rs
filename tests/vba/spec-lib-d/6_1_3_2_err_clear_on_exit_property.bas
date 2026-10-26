' INV: vbal.6.1.3.2.1.1
' SOURCE: spec ms-vbal 6.1.3.2.1.1 Err.Clear is called automatically by Exit Property
' EXPECT: inside=5
' EXPECT: after=0
Private Property Get Prop() As Long
    On Error GoTo h
    Err.Raise 5
    Exit Property
h:
    Debug.Print "inside=" & CStr(Err.Number)
    Exit Property
End Property
Public Sub Main()
    Dim x As Long
    x = Prop
    Debug.Print "after=" & CStr(Err.Number)
End Sub
