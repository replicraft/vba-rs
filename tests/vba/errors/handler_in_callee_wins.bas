' INV: stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.1 On Error statement, 5.4.4 Goto policy
' EXPECT: inner 13
' EXPECT: back
Sub Inner()
    On Error GoTo H
    Error 13
    Exit Sub
H:
    Debug.Print "inner " & Err.Number
End Sub
Public Sub Main()
    On Error GoTo H2
    Inner
    Debug.Print "back"
    Exit Sub
H2:
    Debug.Print "outer"
End Sub
