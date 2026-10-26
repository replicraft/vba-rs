' INV: vbal.5.4.4.1, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.1 a later On Error replaces the policy of the current activation
' EXPECT: h2
Public Sub Main()
    On Error GoTo h1
    On Error GoTo h2
    Error 5
    Exit Sub
h1:
    Debug.Print "h1"
    Exit Sub
h2:
    Debug.Print "h2"
End Sub
