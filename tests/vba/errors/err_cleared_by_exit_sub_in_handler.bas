' INV: obj.err, stmt.exit
' SOURCE: spec ms-vbal 6.1.3.2.1.1 Err.Clear is called by Exit Sub
' EXPECT:  0 
Sub Inner()
    On Error GoTo H
    Error 5
    Exit Sub
H:
    Exit Sub
End Sub
Public Sub Main()
    Inner
    Debug.Print Err.Number
End Sub
