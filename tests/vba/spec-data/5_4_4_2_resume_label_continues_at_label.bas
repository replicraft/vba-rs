' INV: vbal.5.4.4.2, stmt.resume
' SOURCE: spec ms-vbal 5.4.4.2 Resume label continues after the label definition
' EXPECT: a
' EXPECT: recovered
Public Sub Main()
    On Error GoTo h
    Debug.Print "a"
    Error 5
    Debug.Print "skipped"
rec:
    Debug.Print "recovered"
    Exit Sub
h:
    Resume rec
End Sub
